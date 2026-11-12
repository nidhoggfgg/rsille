//! Terminal-cell storage, wide-grapheme ownership, and frame comparison.
//!
//! All positions are absolute within the buffer, starting at `(0, 0)`.
//! A wide grapheme has one leading cell and occupied continuation cells.
//! Call [`Buffer::clear`] after presenting a frame to preserve it for diffing
//! and prepare a blank working frame.

use log::info;

use crate::render::{
    area::{Position, Size},
    raster::RasterCell,
    style::Stylized,
    DrawErr,
};

/// A row-major drawing buffer with optional previous-frame storage.
///
/// Dimensions and positions are measured in terminal cells, not UTF-8 bytes.
/// New buffers contain spaces and are marked dirty. Resizing or changing text
/// metrics discards existing content when the requested value changes.
///
/// Raster ownership is allocated lazily in a separate dense table, one slot per
/// buffer cell regardless of the raster's full target size. Frame history can
/// retain a second table; source pixels remain shared COW snapshots. Buffers
/// that only paint text allocate neither table and keep the same [`Cell`] layout.
///
/// ```
/// use rsille::render::{buffer::Buffer, style::Stylized};
///
/// let mut buffer = Buffer::new((4, 1).into());
/// buffer.set((0, 0).into(), Stylized::plain('界'))?;
/// assert!(buffer.is_occupied((1, 0).into()));
/// buffer.clear(); // Save the frame and prepare spaces for the next one.
/// buffer.set((0, 0).into(), Stylized::plain('界'))?;
/// assert_eq!(buffer.diff().unwrap().count(), 0);
/// # Ok::<(), rsille::render::DrawErr>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct Buffer {
    size: Size,
    content: Vec<Cell>,
    previous: Option<Vec<Cell>>,
    raster: Option<Vec<Option<RasterCell>>>,
    previous_raster: Option<Vec<Option<RasterCell>>>,
    unsupported_raster_composition: bool,
    previous_unsupported_raster_composition: bool,
    dirty: bool,
    text_metrics: crate::render::text::TextMetrics,
}

impl Buffer {
    /// Allocates `width * height` space cells with default text metrics.
    pub fn new(size: Size) -> Self {
        Self {
            size,
            content: vec![Cell::space(); size.width as usize * size.height as usize],
            previous: None,
            raster: None,
            previous_raster: None,
            unsupported_raster_composition: false,
            previous_unsupported_raster_composition: false,
            dirty: true, // Mark dirty on creation to force initial render
            text_metrics: crate::render::text::TextMetrics::default(),
        }
    }

    /// Returns the row-major index of `pos`, or `None` outside the buffer.
    pub fn index(&self, pos: Position) -> Option<usize> {
        if pos.x >= self.size.width || pos.y >= self.size.height {
            return None;
        }

        Some(pos.y as usize * self.size.width as usize + pos.x as usize)
    }

    /// Returns whether `pos` is a continuation cell of a wide grapheme.
    ///
    /// Leading cells and out-of-bounds positions return `false`.
    pub fn is_occupied(&self, pos: Position) -> bool {
        if let Some(i) = self.index(pos) {
            self.content[i].is_occupied
        } else {
            false
        }
    }

    /// Reads a leading cell or space at `pos`.
    ///
    /// Returns `None` for continuation cells and out-of-bounds positions.
    pub fn get(&self, pos: Position) -> Option<&Stylized> {
        if let Some(i) = self.index(pos) {
            if self.content[i].is_occupied {
                None
            } else {
                Some(&self.content[i].content)
            }
        } else {
            None
        }
    }

    /// Returns the current raster owner at an absolute cell position.
    pub fn raster_at(&self, pos: Position) -> Option<&RasterCell> {
        self.raster.as_ref()?.get(self.index(pos)?)?.as_ref()
    }

    /// Returns the previous frame's raster owner at an absolute cell position.
    pub fn previous_raster_at(&self, pos: Position) -> Option<&RasterCell> {
        self.previous_raster
            .as_ref()?
            .get(self.index(pos)?)?
            .as_ref()
    }

    /// Whether the current frame contains any visible native raster cells.
    pub fn has_raster(&self) -> bool {
        self.raster_cells().next().is_some()
    }

    /// Iterates current raster owners in row-major order, in buffer coordinates.
    pub fn raster_cells(&self) -> impl Iterator<Item = (u16, u16, &RasterCell)> {
        self.raster
            .iter()
            .flatten()
            .enumerate()
            .filter_map(|(i, cell)| {
                cell.as_ref().map(|cell| {
                    (
                        (i % self.size.width as usize) as u16,
                        (i / self.size.width as usize) as u16,
                        cell,
                    )
                })
            })
    }

    /// Whether raster content was flattened by an unsupported composition.
    ///
    /// This conservative frame-level marker survives subsequent writes, even
    /// if they cover the affected cells. Strict raster presenters must reject
    /// such frames rather than silently discarding the image.
    pub fn has_unsupported_raster_composition(&self) -> bool {
        self.unsupported_raster_composition
    }

    /// Returns the unsupported-composition marker saved with the previous frame.
    pub fn previous_has_unsupported_raster_composition(&self) -> bool {
        self.previous_unsupported_raster_composition
    }

    /// Marks the current frame as containing flattened raster composition.
    pub fn mark_unsupported_raster_composition(&mut self) {
        self.unsupported_raster_composition = true;
        self.dirty = true;
    }

    /// Paints an opaque background placeholder and attaches its raster owner.
    ///
    /// Replaces intersecting wide graphemes in full, without inheriting styles.
    /// The placeholder is always one space, independent of text-width policy.
    ///
    /// # Errors
    ///
    /// Returns [`DrawErr::OutOfBounds`] for an invalid destination or offsets
    /// outside `cell.target`. Invalid writes do not modify the buffer.
    pub fn set_raster(&mut self, pos: Position, cell: RasterCell) -> Result<(), DrawErr> {
        self.validate_raster(pos, &cell)?;
        self.write(pos, cell.placeholder(), false)?;
        let index = self.index(pos).unwrap();
        self.raster
            .get_or_insert_with(|| vec![None; self.content.len()])[index] = Some(cell);
        Ok(())
    }

    fn validate_raster(&self, pos: Position, cell: &RasterCell) -> Result<(), DrawErr> {
        if self.index(pos).is_none() {
            return Err(DrawErr::out_of_bounds(pos, self.size));
        }
        if cell.x >= cell.target.width || cell.y >= cell.target.height {
            return Err(DrawErr::out_of_bounds((cell.x, cell.y).into(), cell.target));
        }
        Ok(())
    }

    /// Returns the width and tab-stop policy used for writes.
    pub fn text_metrics(&self) -> crate::render::text::TextMetrics {
        self.text_metrics
    }

    /// Change the policy before drawing; clears content and forces a full redraw.
    pub fn set_text_metrics(&mut self, metrics: crate::render::text::TextMetrics) {
        if self.text_metrics != metrics {
            self.text_metrics = metrics;
            self.clear_content();
            self.dirty = true;
        }
    }

    /// Writes content at `pos`, returning the number of terminal columns used.
    ///
    /// Existing leading cells are replaced without style inheritance. Any
    /// intersecting wide grapheme is erased in full. Tabs expand to spaces;
    /// zero-width content leaves existing cells unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`DrawErr::PositionOccupied`] when `pos` is a continuation cell,
    /// or [`DrawErr::OutOfBounds`] if the position or content exceeds the buffer.
    pub fn set(&mut self, pos: Position, content: Stylized) -> Result<usize, DrawErr> {
        if self.is_occupied(pos) {
            return Err(DrawErr::position_occupied(pos));
        }
        self.write(pos, content, false)
    }

    /// Writes over any cell, merging styles, and returns the display width.
    ///
    /// Erases intersecting wide graphemes completely. Missing style components
    /// inherit from the grapheme owning the destination cell. Zero-width
    /// content leaves the buffer unchanged; tabs expand to spaces.
    ///
    /// # Errors
    ///
    /// Returns [`DrawErr::OutOfBounds`] if the position or content does not fit.
    pub fn overwrite(&mut self, pos: Position, content: Stylized) -> Result<usize, DrawErr> {
        self.write(pos, content, true)
    }

    /// Clear the complete owner of any destination cell, including its tail.
    fn erase_owner(&mut self, index: usize) {
        let owner = self.content[index].owner.unwrap_or(index);
        let end = (owner + self.content[owner].width().max(1)).min(self.content.len());
        for cell in &mut self.content[owner..end] {
            *cell = Cell::space();
        }
        if let Some(raster) = &mut self.raster {
            raster[owner..end].fill(None);
        }
    }

    fn write(
        &mut self,
        pos: Position,
        mut content: Stylized,
        merge: bool,
    ) -> Result<usize, DrawErr> {
        let i = self
            .index(pos)
            .ok_or_else(|| DrawErr::out_of_bounds(pos, self.size))?;
        let width = content.width_with(self.text_metrics, pos.x as usize);
        if pos.x as usize + width > self.size.width as usize {
            return Err(DrawErr::out_of_bounds(pos, self.size));
        }
        // Zero-width/empty/control cells never erase an existing glyph.
        if width == 0 {
            return Ok(0);
        }
        let style = self.content[self.content[i].owner.unwrap_or(i)]
            .content
            .style;
        for index in i..i + width {
            self.erase_owner(index);
        }
        if merge {
            content.style = style.merge(content.style);
        }
        // Tabs are expanded by Chunk; direct writes follow the same rule.
        if content.as_char() == Some('\t') {
            for cell in &mut self.content[i..i + width] {
                *cell = Cell::new(Stylized::new(' ', content.style));
            }
        } else {
            self.content[i] = Cell::new(content.clone());
            self.content[i].columns = width;
            for j in i + 1..i + width {
                self.content[j] = Cell {
                    content: Stylized::empty().with_style(content.style),
                    is_occupied: true,
                    owner: Some(i),
                    columns: 0,
                };
            }
        }
        self.dirty = true;
        Ok(width)
    }

    /// Returns the buffer dimensions in terminal cells.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Returns all current cells in row-major order, including continuation cells.
    pub fn content(&self) -> &[Cell] {
        &self.content
    }

    /// Estimates bytes reserved by current and previous cell-storage vectors.
    ///
    /// Includes raster ownership tables and spare vector capacity, but excludes
    /// this struct, allocator overhead, and separately allocated graphemes and
    /// shared source pixels. Pool callers must first call [`Self::clear_content`]
    /// to release those payloads before applying a retained-storage budget.
    #[doc(hidden)]
    pub fn storage_bytes(&self) -> usize {
        let text_cells = self
            .content
            .capacity()
            .saturating_add(self.previous.as_ref().map_or(0, Vec::capacity));
        let raster_cells = self
            .raster
            .as_ref()
            .map_or(0, Vec::capacity)
            .saturating_add(self.previous_raster.as_ref().map_or(0, Vec::capacity));
        text_cells
            .saturating_mul(std::mem::size_of::<Cell>())
            .saturating_add(raster_cells.saturating_mul(std::mem::size_of::<Option<RasterCell>>()))
    }

    /// Clear the buffer for the next frame, preserving previous for diffing
    pub fn clear(&mut self) {
        // Use swap instead of clone to reuse allocated memory
        if self.previous.is_none() {
            // Only allocate if previous doesn't exist
            self.previous = Some(vec![Cell::space(); self.content.len()]);
        }
        // Swap buffers to reuse allocated memory
        std::mem::swap(&mut self.content, self.previous.as_mut().unwrap());
        // Reset all cells in the now-current buffer to empty
        self.content.fill(Cell::space());
        std::mem::swap(&mut self.raster, &mut self.previous_raster);
        if let Some(raster) = &mut self.raster {
            raster.fill(None);
        }
        self.previous_unsupported_raster_composition = self.unsupported_raster_composition;
        self.unsupported_raster_composition = false;
        // Reset dirty flag after saving state
        self.dirty = false;
    }

    /// Clear the current buffer content without preserving diff state.
    ///
    /// This is useful for transient offscreen buffers where callers want to
    /// reuse allocation but do not need frame-to-frame diffing.
    pub fn clear_content(&mut self) {
        self.content.fill(Cell::space());
        if let Some(raster) = &mut self.raster {
            raster.fill(None);
        }
        self.unsupported_raster_composition = false;
        self.reset_diff();
        self.dirty = false;
    }

    /// Clips a staged inline frame without discarding the last displayed diff.
    /// Raster sampling retains its original target, just as with chunk clipping.
    pub(crate) fn clip_inline_rows(&mut self, height: u16) {
        let start = usize::from(height.min(self.size.height)) * usize::from(self.size.width);
        self.content[start..].fill(Cell::space());
        if let Some(raster) = &mut self.raster {
            raster[start..].fill(None);
        }
    }

    /// Discards the previous frame without changing current content or dirtiness.
    pub fn reset_diff(&mut self) {
        self.previous = None;
        self.previous_raster = None;
        self.previous_unsupported_raster_composition = false;
    }

    /// Get the previous buffer state for diffing
    pub fn previous(&self) -> Option<&[Cell]> {
        self.previous.as_deref()
    }

    /// Returns the dirty flag set by writes, resizes, or text-policy changes.
    ///
    /// This flag does not compare frames; writing identical content still marks
    /// the buffer dirty. [`clear`](Self::clear) resets it after saving a frame.
    pub fn has_pending_changes(&self) -> bool {
        self.dirty
    }

    /// Replaces content with spaces and discards the previous frame on resize.
    ///
    /// Marks the buffer dirty when dimensions change; identical dimensions
    /// leave content and flags unchanged. Text metrics are preserved.
    pub fn resize(&mut self, new_size: Size) {
        if self.size == new_size {
            return;
        }

        info!(
            target: "render::buffer",
            "buffer resize: {}x{} -> {}x{}",
            self.size.width, self.size.height,
            new_size.width, new_size.height
        );

        self.size = new_size;
        self.content = vec![Cell::space(); new_size.width as usize * new_size.height as usize];
        self.raster = None;
        self.unsupported_raster_composition = false;
        // Clear previous buffer on resize to force full redraw
        self.reset_diff();
        self.dirty = true; // Mark dirty on resize
    }

    /// Returns an iterator over cells that changed since the previous frame
    ///
    /// Returns `None` when no previous frame exists. Items are `(x, y, cell)`
    /// in row-major order, excluding occupied continuation cells.
    pub fn diff(&self) -> Option<DiffIterator<'_>> {
        self.previous.as_ref().map(|prev| DiffIterator {
            current: &self.content,
            previous: prev,
            raster: self.raster.as_deref(),
            previous_raster: self.previous_raster.as_deref(),
            index: 0,
            width: self.size.width,
        })
    }

    /// Returns an iterator over all visible cells (for first render or full redraw)
    pub fn all_cells(&self) -> AllCellsIterator<'_> {
        AllCellsIterator {
            content: &self.content,
            index: 0,
            width: self.size.width,
        }
    }

    /// Returns an iterator over lines with their change state
    /// Suitable for inline mode rendering with relative positioning
    pub fn diff_lines(&self) -> LineDiffIterator<'_> {
        LineDiffIterator {
            current: &self.content,
            previous: self.previous.as_deref(),
            raster: self.raster.as_deref(),
            previous_raster: self.previous_raster.as_deref(),
            width: self.size.width,
            height: self.size.height,
            line_index: 0,
        }
    }
}

fn raster_cell_at(cells: Option<&[Option<RasterCell>]>, index: usize) -> Option<&RasterCell> {
    cells?.get(index)?.as_ref()
}

/// Changed leading cells as `(column, row, cell)` in row-major order.
///
/// Created by [`Buffer::diff`]; occupied continuation cells are skipped.
pub struct DiffIterator<'a> {
    current: &'a [Cell],
    previous: &'a [Cell],
    raster: Option<&'a [Option<RasterCell>]>,
    previous_raster: Option<&'a [Option<RasterCell>]>,
    index: usize,
    width: u16,
}

impl<'a> Iterator for DiffIterator<'a> {
    type Item = (u16, u16, &'a Cell);

    fn next(&mut self) -> Option<Self::Item> {
        // Keep raster lookups out of the scan for buffers that only paint text.
        if self.raster.is_none() && self.previous_raster.is_none() {
            while self.index < self.current.len() {
                let i = self.index;
                self.index += 1;

                let curr = &self.current[i];
                let prev = &self.previous[i];

                if curr != prev && !curr.is_occupied {
                    let x = (i % self.width as usize) as u16;
                    let y = (i / self.width as usize) as u16;
                    return Some((x, y, curr));
                }
            }
            return None;
        }

        while self.index < self.current.len() {
            let i = self.index;
            self.index += 1;

            let curr = &self.current[i];
            let prev = &self.previous[i];

            // Only yield cells that changed and are not occupied by wide characters
            let raster_changed =
                raster_cell_at(self.raster, i) != raster_cell_at(self.previous_raster, i);
            if (curr != prev || raster_changed) && !curr.is_occupied {
                let x = (i % self.width as usize) as u16;
                let y = (i / self.width as usize) as u16;
                return Some((x, y, curr));
            }
        }
        None
    }
}

/// All leading cells and spaces as `(column, row, cell)` in row-major order.
///
/// Created by [`Buffer::all_cells`]; occupied continuation cells are skipped.
pub struct AllCellsIterator<'a> {
    content: &'a [Cell],
    index: usize,
    width: u16,
}

impl<'a> Iterator for AllCellsIterator<'a> {
    type Item = (u16, u16, &'a Cell);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.content.len() {
            let i = self.index;
            self.index += 1;

            let cell = &self.content[i];

            // Only yield cells that are not occupied by wide characters
            if !cell.is_occupied {
                let x = (i % self.width as usize) as u16;
                let y = (i / self.width as usize) as u16;
                return Some((x, y, cell));
            }
        }
        None
    }
}

/// Iterator over visible cells in a single line, skipping occupied cells
#[derive(Debug, Clone)]
pub struct LineCellsIterator<'a> {
    cells: &'a [Cell],
    index: usize,
}

impl<'a> Iterator for LineCellsIterator<'a> {
    type Item = &'a Cell;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.cells.len() {
            let cell = &self.cells[self.index];
            self.index += 1;

            // Only yield cells that are not occupied by wide characters
            if !cell.is_occupied {
                return Some(cell);
            }
        }
        None
    }
}

/// Represents the state of a line when comparing with previous frame
#[derive(Debug)]
pub enum LineState<'a> {
    /// Line content has not changed, no need to re-render
    Unchanged,
    /// Line content has changed, provides cells and render width info
    Changed {
        /// Iterator over visible cells in the line
        cells: LineCellsIterator<'a>,
        /// Actual render width of current line (sum of cell widths, e.g., wide chars count as 2)
        current_len: usize,
        /// Actual render width of previous line (sum of cell widths)
        previous_len: usize,
    },
}

/// A line with its change state
#[derive(Debug)]
pub struct LineDiff<'a> {
    /// Line number (0-indexed)
    pub line_num: u16,
    /// Change state of the line
    pub state: LineState<'a>,
}

/// Iterator over lines, comparing current with previous frame
pub struct LineDiffIterator<'a> {
    current: &'a [Cell],
    previous: Option<&'a [Cell]>,
    raster: Option<&'a [Option<RasterCell>]>,
    previous_raster: Option<&'a [Option<RasterCell>]>,
    width: u16,
    height: u16,
    line_index: usize,
}

impl<'a> Iterator for LineDiffIterator<'a> {
    type Item = LineDiff<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.line_index >= self.height as usize {
            return None;
        }

        let line_num = self.line_index as u16;
        let start = self.line_index * self.width as usize;
        let end = start + self.width as usize;
        self.line_index += 1;

        // Get current line slice
        let current_line = &self.current[start..end.min(self.current.len())];

        // Determine line state by comparing with previous
        let state = if let Some(prev) = self.previous {
            let prev_line = &prev[start..end.min(prev.len())];

            // Compare lines
            let raster_unchanged = (self.raster.is_none() && self.previous_raster.is_none())
                || (start..end).all(|i| {
                    raster_cell_at(self.raster, i) == raster_cell_at(self.previous_raster, i)
                });
            if current_line == prev_line && raster_unchanged {
                LineState::Unchanged
            } else {
                // Calculate actual render width (sum of cell widths)
                let current_len = current_line
                    .iter()
                    .filter(|c| !c.is_occupied)
                    .map(|c| c.width())
                    .sum::<usize>();
                let previous_len = prev_line
                    .iter()
                    .filter(|c| !c.is_occupied)
                    .map(|c| c.width())
                    .sum::<usize>();

                LineState::Changed {
                    cells: LineCellsIterator {
                        cells: current_line,
                        index: 0,
                    },
                    current_len,
                    previous_len,
                }
            }
        } else {
            // First render, all lines are "changed"
            let current_len = current_line
                .iter()
                .filter(|c| !c.is_occupied)
                .map(|c| c.width())
                .sum::<usize>();

            LineState::Changed {
                cells: LineCellsIterator {
                    cells: current_line,
                    index: 0,
                },
                current_len,
                previous_len: 0,
            }
        };

        Some(LineDiff { line_num, state })
    }
}

/// One buffer slot, holding a grapheme or its continuation metadata.
///
/// [`width`](Self::width) is cached when the cell is created or written.
/// Mutating public fields directly does not recompute this cache. The default
/// is empty with zero width; [`space`](Self::space) creates a blank column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cell {
    /// Styled grapheme, or an empty symbol in a continuation cell.
    pub content: Stylized,
    /// Whether this slot belongs to a wide grapheme in an earlier column.
    pub is_occupied: bool,
    /// Row-major index of the leading cell for a continuation slot.
    pub owner: Option<usize>,
    columns: usize,
}

impl Cell {
    /// Creates a leading cell from an unstyled character.
    pub fn raw(c: char) -> Self {
        Self::new(Stylized::plain(c))
    }

    /// Creates a leading cell with width measured using default text metrics.
    pub fn new(content: Stylized) -> Self {
        let columns = content.width();
        Self {
            content,
            is_occupied: false,
            owner: None,
            columns,
        }
    }

    /// Creates an unstyled space occupying one column.
    pub fn space() -> Self {
        Self {
            content: Stylized::space(),
            is_occupied: false,
            owner: None,
            columns: 1,
        }
    }

    /// Returns the cached column count; continuation cells have width zero.
    pub fn width(&self) -> usize {
        self.columns
    }

    /// Measures the content with ambiguous characters treated as wide.
    ///
    /// Unlike [`width`](Self::width), this recomputes width from the symbol.
    pub fn width_cjk(&self) -> usize {
        self.content.width_cjk()
    }

    /// Queues styled content without flushing, propagating writer errors.
    ///
    /// Delegates to [`Stylized::queue`], which skips empty and zero-width symbols.
    pub fn queue(&self, buffer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.content.queue(buffer)
    }

    /// Returns whether the content has explicit color settings.
    pub fn has_color(&self) -> bool {
        self.content.has_color()
    }

    /// Returns whether the content has explicit text attributes.
    pub fn has_attr(&self) -> bool {
        self.content.has_attr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::style::Style;
    use crossterm::style::{Color, Colors};

    fn raster_cell() -> RasterCell {
        RasterCell {
            canvas: crate::render::raster::RasterCanvas::new(2, 2, Default::default()).unwrap(),
            target: (2, 1).into(),
            x: 0,
            y: 0,
            options: Default::default(),
        }
    }

    #[test]
    fn raster_metadata_footprint() {
        use std::mem::size_of;
        // Keep the dense owner table out of Cell: text frames only pay the
        // fixed Buffer fields, while raster frames share their source pixels.
        println!("Cell={} Buffer={} RasterCanvas={} RasterCell={} owner_slot={} metadata_120x40_two_frames={}",
            size_of::<Cell>(), size_of::<Buffer>(),
            size_of::<crate::render::raster::RasterCanvas>(), size_of::<RasterCell>(),
            size_of::<Option<RasterCell>>(), 2 * 120 * 40 * size_of::<Option<RasterCell>>());
        let buffer = Buffer::new((120, 40).into());
        assert!(buffer.raster.is_none());
        assert!(buffer.previous_raster.is_none());
    }

    #[test]
    fn text_occludes_native_raster_with_explicit_or_inherited_background() {
        for background in [None, Some(Color::Blue)] {
            let mut buffer = Buffer::new((3, 1).into());
            let cell = raster_cell();
            for x in 0..3 {
                buffer.set_raster((x, 0).into(), cell.clone()).unwrap();
            }
            let inherited = buffer
                .get((1, 0).into())
                .unwrap()
                .style
                .colors
                .unwrap()
                .background;
            buffer
                .overwrite(
                    (1, 0).into(),
                    Stylized::new(
                        'x',
                        Style::with_colors(Colors {
                            foreground: Some(Color::White),
                            background,
                        }),
                    ),
                )
                .unwrap();
            assert!(buffer.raster_at((1, 0).into()).is_none());
            assert_eq!(buffer.raster_cells().count(), 2);
            assert_eq!(buffer.raster_at((0, 0).into()), Some(&cell));
            assert_eq!(buffer.raster_at((2, 0).into()), Some(&cell));
            assert_eq!(
                buffer
                    .get((1, 0).into())
                    .unwrap()
                    .style
                    .colors
                    .unwrap()
                    .background,
                background.or(inherited)
            );
            assert!(!buffer.has_unsupported_raster_composition());
        }
    }

    #[test]
    fn storage_bytes_counts_capacity_and_clear_releases_frame_payloads() {
        use std::mem::size_of;
        let mut buffer = Buffer::new((2, 1).into());
        assert_eq!(
            buffer.storage_bytes(),
            buffer.content.capacity() * size_of::<Cell>()
        );
        buffer.set_raster((0, 0).into(), raster_cell()).unwrap();
        buffer.clear();
        buffer.set_raster((1, 0).into(), raster_cell()).unwrap();
        buffer.content.reserve(5);
        buffer.previous.as_mut().unwrap().reserve(3);
        buffer.raster.as_mut().unwrap().reserve(4);
        let current_bytes = buffer.content.capacity() * size_of::<Cell>()
            + buffer.raster.as_ref().unwrap().capacity() * size_of::<Option<RasterCell>>();
        let previous_bytes = buffer.previous.as_ref().unwrap().capacity() * size_of::<Cell>()
            + buffer.previous_raster.as_ref().unwrap().capacity() * size_of::<Option<RasterCell>>();
        assert_eq!(buffer.storage_bytes(), current_bytes + previous_bytes);
        buffer.clear_content();
        assert_eq!(buffer.storage_bytes(), current_bytes);
        assert!(buffer.raster.as_ref().unwrap().iter().all(Option::is_none));
        assert!(buffer.previous_raster.is_none());
        assert!(buffer.previous.is_none());
        assert!(buffer.content.iter().all(|cell| *cell == Cell::space()));
    }

    #[test]
    fn raster_storage_is_lazy_for_text_frames() {
        let mut buffer = Buffer::new((3, 1).into());
        buffer.set((0, 0).into(), Stylized::plain('x')).unwrap();
        buffer.clear();
        buffer.clear();
        buffer.clear_content();
        buffer.resize((4, 2).into());
        assert!(buffer.raster.is_none());
        assert!(buffer.previous_raster.is_none());
        assert!(!buffer.has_raster());
        assert_eq!(Buffer::default().raster_cells().count(), 0);
    }

    #[test]
    fn raster_lifecycle_tracks_current_and_previous_frames() {
        let mut buffer = Buffer::new((3, 1).into());
        let cell = raster_cell();
        buffer.set_raster((1, 0).into(), cell.clone()).unwrap();
        buffer.mark_unsupported_raster_composition();
        let snapshot = buffer.clone();
        buffer.clear();
        assert!(!buffer.has_raster());
        assert!(!buffer.has_pending_changes());
        assert_eq!(buffer.previous_raster_at((1, 0).into()), Some(&cell));
        assert!(!buffer.has_unsupported_raster_composition());
        assert!(buffer.previous_has_unsupported_raster_composition());
        assert_eq!(snapshot.raster_at((1, 0).into()), Some(&cell));
        buffer.set_raster((1, 0).into(), cell.clone()).unwrap();
        assert_eq!(buffer.diff().unwrap().count(), 0);
        buffer.reset_diff();
        assert_eq!(buffer.raster_at((1, 0).into()), Some(&cell));
        assert!(buffer.previous_raster_at((1, 0).into()).is_none());
        assert!(!buffer.previous_has_unsupported_raster_composition());
        buffer.clear();
        buffer.clear();
        assert!(buffer.previous_raster_at((1, 0).into()).is_none());
        assert!(!buffer.has_raster());
    }

    #[test]
    fn raster_clear_resize_and_metrics_discard_owners() {
        use crate::render::text::{AmbiguousWidth, TextMetrics};
        let mut buffer = Buffer::new((3, 1).into());
        for action in 0..3 {
            buffer.set_raster((0, 0).into(), raster_cell()).unwrap();
            buffer.mark_unsupported_raster_composition();
            buffer.clear();
            buffer.set_raster((1, 0).into(), raster_cell()).unwrap();
            buffer.mark_unsupported_raster_composition();
            match action {
                0 => buffer.clear_content(),
                1 => {
                    buffer.resize(buffer.size());
                    assert!(buffer.has_raster());
                    buffer.resize((4, 2).into());
                }
                _ => buffer.set_text_metrics(TextMetrics {
                    ambiguous_width: AmbiguousWidth::Wide,
                    ..TextMetrics::default()
                }),
            }
            assert!(!buffer.has_raster());
            assert!(buffer.previous().is_none());
            assert!(buffer.previous_raster.is_none());
            assert!(!buffer.has_unsupported_raster_composition());
            assert!(!buffer.previous_has_unsupported_raster_composition());
        }
    }

    #[test]
    fn raster_overwrites_and_text_rewrites_respect_wide_owners() {
        let mut buffer = Buffer::new((4, 1).into());
        buffer.set((0, 0).into(), Stylized::plain('界')).unwrap();
        let cell = raster_cell();
        buffer.set_raster((1, 0).into(), cell.clone()).unwrap();
        assert_eq!(buffer.get((0, 0).into()).unwrap().as_char(), Some(' '));
        assert!(!buffer.is_occupied((1, 0).into()));
        assert_eq!(buffer.raster_at((1, 0).into()), Some(&cell));
        buffer.set_raster((2, 0).into(), cell.clone()).unwrap();
        buffer
            .overwrite((1, 0).into(), Stylized::plain('界'))
            .unwrap();
        assert!(!buffer.has_raster());
        buffer
            .overwrite((2, 0).into(), Stylized::plain('x'))
            .unwrap();
        assert_eq!(buffer.get((1, 0).into()).unwrap().as_char(), Some(' '));
        for merge in [false, true] {
            buffer.set_raster((3, 0).into(), cell.clone()).unwrap();
            let placeholder = buffer.get((3, 0).into()).unwrap().clone();
            if merge {
                buffer.overwrite((3, 0).into(), placeholder).unwrap();
            } else {
                buffer.set((3, 0).into(), placeholder).unwrap();
            }
            assert!(buffer.raster_at((3, 0).into()).is_none());
        }
    }

    #[test]
    fn text_diff_fast_path_matches_empty_raster_tables() {
        let mut buffer = Buffer::new((6, 2).into());
        buffer.set((0, 0).into(), Stylized::plain('a')).unwrap();
        buffer.set((1, 0).into(), Stylized::plain('界')).unwrap();
        buffer.clear();
        buffer.set((0, 0).into(), Stylized::plain('b')).unwrap();
        buffer.set((3, 0).into(), Stylized::plain('界')).unwrap();
        buffer.set((5, 1).into(), Stylized::plain('z')).unwrap();
        assert!(buffer.raster.is_none() && buffer.previous_raster.is_none());
        let expected: Vec<_> = buffer
            .diff()
            .unwrap()
            .map(|(x, y, cell)| (x, y, cell.clone()))
            .collect();
        assert_eq!(
            expected
                .iter()
                .map(|(x, y, _)| (*x, *y))
                .collect::<Vec<_>>(),
            vec![(0, 0), (1, 0), (2, 0), (3, 0), (5, 1)]
        );
        for (current, previous) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut with_tables = buffer.clone();
            with_tables.raster = current.then(|| vec![None; buffer.content.len()]);
            with_tables.previous_raster = previous.then(|| vec![None; buffer.content.len()]);
            let mut diff = with_tables.diff().unwrap();
            assert_eq!(
                diff.by_ref()
                    .map(|(x, y, cell)| (x, y, cell.clone()))
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(diff.next().is_none());
            assert!(diff.next().is_none());
        }
    }

    #[test]
    fn raster_metadata_only_changes_are_dirty() {
        let mut buffer = Buffer::new((2, 1).into());
        let first = raster_cell();
        let mut second = first.clone();
        second.x = 1;
        assert_eq!(first.placeholder(), second.placeholder());
        buffer.set_raster((0, 0).into(), first).unwrap();
        buffer.clear();
        buffer.set_raster((0, 0).into(), second).unwrap();
        assert_eq!(
            buffer
                .diff()
                .unwrap()
                .map(|(x, y, _)| (x, y))
                .collect::<Vec<_>>(),
            vec![(0, 0)]
        );
        assert!(matches!(
            buffer.diff_lines().next().unwrap().state,
            LineState::Changed { .. }
        ));
        buffer.clear();
        let placeholder = buffer.previous().unwrap()[0].content.clone();
        buffer.set((0, 0).into(), placeholder).unwrap();
        assert_eq!(buffer.diff().unwrap().count(), 1);
    }

    #[test]
    fn raster_target_bounds_do_not_size_the_ownership_allocation() {
        let mut buffer = Buffer::new((1, 1).into());
        for target in [(0, 1), (1, 0), (0, 0)] {
            let mut cell = raster_cell();
            cell.target = target.into();
            assert!(buffer.set_raster((0, 0).into(), cell).is_err());
            assert!(buffer.raster.is_none());
        }
        let mut cell = raster_cell();
        cell.target = (u16::MAX, u16::MAX).into();
        cell.x = u16::MAX - 1;
        cell.y = u16::MAX - 1;
        buffer.set_raster((0, 0).into(), cell.clone()).unwrap();
        assert_eq!(buffer.raster.as_ref().unwrap().len(), 1);
        assert_eq!(buffer.raster_at((0, 0).into()), Some(&cell));
        cell.y = u16::MAX;
        assert!(buffer.set_raster((0, 0).into(), cell).is_err());
        assert_eq!(buffer.raster_at((0, 0).into()).unwrap().y, u16::MAX - 1);
    }

    #[test]
    fn raster_invalid_writes_are_atomic_and_placeholder_is_opaque() {
        let mut buffer = Buffer::new((2, 1).into());
        buffer.set((0, 0).into(), Stylized::plain('界')).unwrap();
        let before = buffer.content().to_vec();
        let mut invalid = raster_cell();
        invalid.x = invalid.target.width;
        assert!(buffer.set_raster((0, 0).into(), invalid).is_err());
        assert!(buffer.set_raster((2, 0).into(), raster_cell()).is_err());
        assert_eq!(buffer.content(), before);
        assert!(buffer.raster.is_none());
        let cell = raster_cell();
        buffer.set_raster((0, 0).into(), cell.clone()).unwrap();
        assert_eq!(buffer.get((0, 0).into()), Some(&cell.placeholder()));
        assert!(buffer
            .get((0, 0).into())
            .unwrap()
            .style
            .colors
            .unwrap()
            .background
            .is_some());
        assert!(buffer.raster_at((u16::MAX, u16::MAX).into()).is_none());
    }

    #[test]
    fn test_buffer_resize() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // Initial size should be 10x10
        assert_eq!(
            buffer.size(),
            Size {
                width: 10,
                height: 10
            }
        );
        assert_eq!(buffer.content.len(), 100);

        // Resize to 20x15
        buffer.resize(Size {
            width: 20,
            height: 15,
        });

        // Check new size
        assert_eq!(
            buffer.size(),
            Size {
                width: 20,
                height: 15
            }
        );
        assert_eq!(buffer.content.len(), 300);

        // Previous buffer should be cleared on resize
        assert!(buffer.previous().is_none());
    }

    #[test]
    fn test_buffer_resize_same_size() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });
        buffer.clear(); // Create a previous buffer

        assert!(buffer.previous().is_some());

        // Resize to same size should do nothing
        buffer.resize(Size {
            width: 10,
            height: 10,
        });

        // Previous buffer should still exist
        assert!(buffer.previous().is_some());
    }

    #[test]
    fn test_diff_no_previous() {
        let buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // No previous buffer, should return None
        assert!(buffer.diff().is_none());
    }

    #[test]
    fn test_diff_no_changes() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // Draw some content
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('b'))
            .unwrap();

        // Save as previous
        buffer.clear();

        // Draw the same content
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('b'))
            .unwrap();

        // Should have no differences
        let diff: Vec<_> = buffer.diff().unwrap().collect();
        assert_eq!(diff.len(), 0);
    }

    #[test]
    fn overwrite_preserves_existing_background_when_overlay_only_sets_foreground() {
        let mut buffer = Buffer::new(Size {
            width: 1,
            height: 1,
        });

        let base_style = Style::with_colors(Colors {
            foreground: None,
            background: Some(Color::Blue),
        });
        let text_style = Style::with_colors(Colors {
            foreground: Some(Color::White),
            background: None,
        });

        buffer
            .overwrite(Position { x: 0, y: 0 }, Stylized::new(' ', base_style))
            .unwrap();
        buffer
            .overwrite(Position { x: 0, y: 0 }, Stylized::new('H', text_style))
            .unwrap();

        let cell = &buffer.content()[0];
        let colors = cell.content.style.colors.unwrap();
        assert_eq!(cell.content.as_char(), Some('H'));
        assert_eq!(colors.foreground, Some(Color::White));
        assert_eq!(colors.background, Some(Color::Blue));
    }

    #[test]
    fn test_diff_with_changes() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // Draw initial content
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('b'))
            .unwrap();
        buffer
            .set(Position { x: 2, y: 0 }, Stylized::plain('c'))
            .unwrap();

        // Save as previous
        buffer.clear();

        // Draw new content with changes
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap(); // Same
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('X'))
            .unwrap(); // Changed
        buffer
            .set(Position { x: 2, y: 0 }, Stylized::plain('c'))
            .unwrap(); // Same
        buffer
            .set(Position { x: 3, y: 0 }, Stylized::plain('d'))
            .unwrap(); // New

        // Should have 2 differences: position (1,0) and (3,0)
        let diff: Vec<_> = buffer.diff().unwrap().collect();
        assert_eq!(diff.len(), 2);

        // Check changed cell at (1, 0)
        assert!(diff
            .iter()
            .any(|(x, y, cell)| { *x == 1 && *y == 0 && cell.content.as_char() == Some('X') }));

        // Check new cell at (3, 0)
        assert!(diff
            .iter()
            .any(|(x, y, cell)| { *x == 3 && *y == 0 && cell.content.as_char() == Some('d') }));
    }

    #[test]
    fn test_all_cells() {
        let mut buffer = Buffer::new(Size {
            width: 3,
            height: 2,
        });

        // Draw content in specific positions
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('b'))
            .unwrap();
        buffer
            .set(Position { x: 0, y: 1 }, Stylized::plain('c'))
            .unwrap();

        let all: Vec<_> = buffer.all_cells().collect();

        // Should have 6 cells total (3x2)
        assert_eq!(all.len(), 6);

        // Verify some cells
        assert!(all
            .iter()
            .any(|(x, y, cell)| { *x == 0 && *y == 0 && cell.content.as_char() == Some('a') }));
        assert!(all
            .iter()
            .any(|(x, y, cell)| { *x == 1 && *y == 0 && cell.content.as_char() == Some('b') }));
        assert!(all
            .iter()
            .any(|(x, y, cell)| { *x == 0 && *y == 1 && cell.content.as_char() == Some('c') }));
    }

    #[test]
    fn test_diff_skips_occupied_cells() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // Draw a wide character (e.g., Chinese character that occupies 2 cells)
        let wide_char = Stylized::plain('中'); // Chinese character, width = 2
        buffer
            .set(Position { x: 0, y: 0 }, wide_char.clone())
            .unwrap();

        // Save as previous
        buffer.clear();

        // Draw different wide character at same position
        let new_wide_char = Stylized::plain('文');
        buffer.set(Position { x: 0, y: 0 }, new_wide_char).unwrap();

        // Should only report the owner cell, not the occupied cell
        let diff: Vec<_> = buffer.diff().unwrap().collect();

        // Should have exactly 1 change (the owner cell at position 0,0)
        // The occupied cell at position 1,0 should be skipped
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].0, 0); // x
        assert_eq!(diff[0].1, 0); // y
    }

    #[test]
    fn test_all_cells_skips_occupied() {
        let mut buffer = Buffer::new(Size {
            width: 5,
            height: 1,
        });

        // Draw: 'a' + wide_char (2 cells) + 'b'
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('中'))
            .unwrap(); // width = 2
        buffer
            .set(Position { x: 3, y: 0 }, Stylized::plain('b'))
            .unwrap();

        let all: Vec<_> = buffer.all_cells().collect();

        // Should have 5 visible cells total:
        // - 'a' at (0, 0)
        // - '中' at (1, 0) - owner
        // - occupied at (2, 0) - skipped
        // - 'b' at (3, 0)
        // - ' ' at (4, 0)
        assert_eq!(all.len(), 4); // 5 total - 1 occupied = 4

        // Verify the occupied cell is not in the results
        assert!(!all
            .iter()
            .any(|(x, y, cell)| { *x == 2 && *y == 0 && cell.is_occupied }));
    }

    #[test]
    fn test_diff_lines_no_previous() {
        let mut buffer = Buffer::new(Size {
            width: 5,
            height: 2,
        });

        // Draw content in first line
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('h'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('i'))
            .unwrap();

        let lines: Vec<_> = buffer.diff_lines().collect();

        // Should have 2 lines
        assert_eq!(lines.len(), 2);

        // First line should be Changed (no previous buffer)
        assert!(matches!(lines[0].state, LineState::Changed { .. }));
        assert_eq!(lines[0].line_num, 0);

        // Second line should also be Changed
        assert!(matches!(lines[1].state, LineState::Changed { .. }));
        assert_eq!(lines[1].line_num, 1);
    }

    #[test]
    fn test_diff_lines_all_unchanged() {
        let mut buffer = Buffer::new(Size {
            width: 5,
            height: 2,
        });

        // Draw content
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        buffer
            .set(Position { x: 0, y: 1 }, Stylized::plain('b'))
            .unwrap();

        // Save as previous
        buffer.clear();

        // Draw the same content
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        buffer
            .set(Position { x: 0, y: 1 }, Stylized::plain('b'))
            .unwrap();

        let lines: Vec<_> = buffer.diff_lines().collect();

        // Both lines should be Unchanged
        assert_eq!(lines.len(), 2);
        assert!(matches!(lines[0].state, LineState::Unchanged));
        assert!(matches!(lines[1].state, LineState::Unchanged));
    }

    #[test]
    fn test_diff_lines_partial_change() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 3,
        });

        // Draw initial content
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('L'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('1'))
            .unwrap();
        buffer
            .set(Position { x: 0, y: 1 }, Stylized::plain('L'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 1 }, Stylized::plain('2'))
            .unwrap();
        buffer
            .set(Position { x: 0, y: 2 }, Stylized::plain('L'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 2 }, Stylized::plain('3'))
            .unwrap();

        // Save as previous
        buffer.clear();

        // Change only line 1 (middle line)
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('L'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('1'))
            .unwrap();
        buffer
            .set(Position { x: 0, y: 1 }, Stylized::plain('X'))
            .unwrap(); // Changed
        buffer
            .set(Position { x: 1, y: 1 }, Stylized::plain('X'))
            .unwrap(); // Changed
        buffer
            .set(Position { x: 0, y: 2 }, Stylized::plain('L'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 2 }, Stylized::plain('3'))
            .unwrap();

        let lines: Vec<_> = buffer.diff_lines().collect();

        assert_eq!(lines.len(), 3);
        assert!(matches!(lines[0].state, LineState::Unchanged)); // Line 0 unchanged
        assert!(matches!(lines[1].state, LineState::Changed { .. })); // Line 1 changed
        assert!(matches!(lines[2].state, LineState::Unchanged)); // Line 2 unchanged
    }

    #[test]
    fn test_diff_lines_shorter_line() {
        let mut buffer = Buffer::new(Size {
            width: 20,
            height: 1,
        });

        // Draw long line
        for i in 0..10 {
            buffer
                .set(Position { x: i, y: 0 }, Stylized::plain('X'))
                .unwrap();
        }

        // Save as previous
        buffer.clear();

        // Draw shorter line
        for i in 0..5 {
            buffer
                .set(Position { x: i, y: 0 }, Stylized::plain('Y'))
                .unwrap();
        }

        let lines: Vec<_> = buffer.diff_lines().collect();

        assert_eq!(lines.len(), 1);
        if let LineState::Changed {
            current_len,
            previous_len,
            ..
        } = &lines[0].state
        {
            // Both lines have 20 visible cells (width of buffer)
            // Previous: 10 'X's + 10 spaces = 20 visible
            // Current: 5 'Y's + 15 spaces = 20 visible
            assert_eq!(*current_len, 20);
            assert_eq!(*previous_len, 20);
        } else {
            panic!("Expected Changed state");
        }
    }

    #[test]
    fn test_diff_lines_longer_line() {
        let mut buffer = Buffer::new(Size {
            width: 20,
            height: 1,
        });

        // Draw short line
        for i in 0..5 {
            buffer
                .set(Position { x: i, y: 0 }, Stylized::plain('X'))
                .unwrap();
        }

        // Save as previous
        buffer.clear();

        // Draw longer line
        for i in 0..10 {
            buffer
                .set(Position { x: i, y: 0 }, Stylized::plain('Y'))
                .unwrap();
        }

        let lines: Vec<_> = buffer.diff_lines().collect();

        assert_eq!(lines.len(), 1);
        if let LineState::Changed {
            current_len,
            previous_len,
            ..
        } = &lines[0].state
        {
            // Both lines have 20 visible cells
            assert_eq!(*current_len, 20);
            assert_eq!(*previous_len, 20);
        } else {
            panic!("Expected Changed state");
        }
    }

    #[test]
    fn test_diff_lines_with_wide_chars() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 1,
        });

        // Draw line with wide character
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('中'))
            .unwrap(); // width = 2

        // Save as previous
        buffer.clear();

        // Change to different wide character
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('文'))
            .unwrap(); // width = 2

        let lines: Vec<_> = buffer.diff_lines().collect();

        assert_eq!(lines.len(), 1);
        assert!(matches!(lines[0].state, LineState::Changed { .. }));

        // Verify the actual render width
        if let LineState::Changed {
            cells,
            current_len,
            previous_len,
            ..
        } = &lines[0].state
        {
            // Width is 10, line has: 'a'(1) + '文'(2) + spaces(7) = 10 total width
            // Previous had: 'a'(1) + '中'(2) + spaces(7) = 10 total width
            assert_eq!(*current_len, 10);
            assert_eq!(*previous_len, 10);

            // Verify that cells iterator skips occupied cells
            let cells_vec: Vec<_> = cells.clone().collect();
            // 9 visible cells: 'a', '文'(owner), 7 spaces (occupied cell is skipped)
            assert_eq!(cells_vec.len(), 9);
        } else {
            panic!("Expected Changed state");
        }
    }

    #[test]
    fn test_diff_lines_render_width_with_wide_chars() {
        let mut buffer = Buffer::new(Size {
            width: 20,
            height: 1,
        });

        // Draw long line with wide characters: "你好世界" = 4 chars × 2 width = 8
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('你'))
            .unwrap();
        buffer
            .set(Position { x: 2, y: 0 }, Stylized::plain('好'))
            .unwrap();
        buffer
            .set(Position { x: 4, y: 0 }, Stylized::plain('世'))
            .unwrap();
        buffer
            .set(Position { x: 6, y: 0 }, Stylized::plain('界'))
            .unwrap();
        // Total render width: 4 × 2 = 8, plus 12 spaces = 20

        // Save as previous
        buffer.clear();

        // Draw shorter line: "Hi" = 2 chars × 1 width = 2
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('H'))
            .unwrap();
        buffer
            .set(Position { x: 1, y: 0 }, Stylized::plain('i'))
            .unwrap();
        // Total render width: 2, plus 18 spaces = 20

        let lines: Vec<_> = buffer.diff_lines().collect();

        assert_eq!(lines.len(), 1);
        if let LineState::Changed {
            current_len,
            previous_len,
            ..
        } = &lines[0].state
        {
            // Current: H(1) + i(1) + 18 spaces = 20 total render width
            assert_eq!(*current_len, 20);
            // Previous: 你(2) + 好(2) + 世(2) + 界(2) + 12 spaces = 20 total render width
            assert_eq!(*previous_len, 20);
        } else {
            panic!("Expected Changed state");
        }
    }

    #[test]
    fn test_dirty_flag_on_creation() {
        let buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // New buffer should be dirty
        assert!(buffer.has_pending_changes());
    }

    #[test]
    fn test_dirty_flag_on_set() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // Clear the dirty flag
        buffer.clear();
        assert!(!buffer.has_pending_changes());

        // Set should mark as dirty
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        assert!(buffer.has_pending_changes());
    }

    #[test]
    fn test_dirty_flag_on_overwrite() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // Clear the dirty flag
        buffer.clear();
        assert!(!buffer.has_pending_changes());

        // Overwrite should mark as dirty
        buffer
            .overwrite(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        assert!(buffer.has_pending_changes());
    }

    #[test]
    fn test_dirty_flag_cleared_after_clear() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // Set content to mark as dirty
        buffer
            .set(Position { x: 0, y: 0 }, Stylized::plain('a'))
            .unwrap();
        assert!(buffer.has_pending_changes());

        // Clear should reset the dirty flag
        buffer.clear();
        assert!(!buffer.has_pending_changes());
    }

    #[test]
    fn test_dirty_flag_on_resize() {
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 10,
        });

        // Clear the dirty flag
        buffer.clear();
        assert!(!buffer.has_pending_changes());

        // Resize should mark as dirty
        buffer.resize(Size {
            width: 20,
            height: 20,
        });
        assert!(buffer.has_pending_changes());
    }

    #[test]
    fn test_swap_optimization_in_clear() {
        let mut buffer = Buffer::new(Size {
            width: 100,
            height: 30,
        });

        // Set some content
        for y in 0..30 {
            for x in 0..100 {
                buffer
                    .set(
                        Position { x, y },
                        Stylized::plain((x as u8 + y as u8) as char),
                    )
                    .unwrap();
            }
        }

        // First clear - previous should be allocated
        buffer.clear();
        assert!(buffer.previous().is_some());
        assert_eq!(buffer.previous().unwrap().len(), 3000);

        // The swap optimization ensures that:
        // 1. Memory is reused instead of allocating new buffers
        // 2. The previous buffer gets swapped with content
        // We can verify the swap happened by checking that previous exists
        // and has the correct size after multiple clears

        // Add more content
        for y in 0..30 {
            for x in 0..100 {
                buffer
                    .set(
                        Position { x, y },
                        Stylized::plain((x as u8 + y as u8 + 1) as char),
                    )
                    .unwrap();
            }
        }

        // Second clear - should swap buffers without new allocation
        buffer.clear();
        assert!(buffer.previous().is_some());
        assert_eq!(buffer.previous().unwrap().len(), 3000);

        // Third clear - verify the pattern continues
        for y in 0..30 {
            for x in 0..100 {
                buffer
                    .set(
                        Position { x, y },
                        Stylized::plain((x as u8 + y as u8 + 2) as char),
                    )
                    .unwrap();
            }
        }

        buffer.clear();
        assert!(buffer.previous().is_some());
        assert_eq!(buffer.previous().unwrap().len(), 3000);

        // The test passes if we got here without panics or reallocations
        // The swap optimization successfully reuses memory
    }
}
