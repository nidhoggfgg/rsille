//! Bounded, mutable views into a drawing buffer.
//!
//! Painting coordinates are local to a [`Chunk`]. Areas passed to constructors
//! use absolute buffer coordinates. Text helpers clip whole graphemes at the
//! right edge and preserve the buffer's text-width policy.

use log::trace;

use crate::render::{
    area::Area,
    buffer::Buffer,
    raster::{RasterCanvas, RasterCell, RasterOptions},
    style::{Style, Stylized},
    DrawErr,
};

/// A mutable drawing view with a local origin and a rectangular boundary.
///
/// [`set`](Self::set) rejects writes starting in a wide grapheme's continuation
/// cell. [`set_forced`](Self::set_forced) can replace it, erasing the entire old
/// grapheme and merging styles. Subchunks borrow the same underlying buffer.
///
/// ```
/// use rsille::render::{area::Size, buffer::Buffer, chunk::Chunk, style::Style};
///
/// let mut buffer = Buffer::new((12, 2).into());
/// let mut chunk = Chunk::new(&mut buffer, Size::from((12, 2)).into())?;
/// chunk.set_string(1, 0, "Hello", Style::default())?;
/// # Ok::<(), rsille::render::DrawErr>(())
/// ```
#[derive(Debug)]
pub struct Chunk<'a> {
    buffer: &'a mut Buffer,
    area: Area,
}

impl<'a> Chunk<'a> {
    /// Borrows the specified area in absolute buffer coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`DrawErr::InvalidArea`] if the area's exclusive end exceeds
    /// the buffer dimensions, including when origin plus size overflows `u16`.
    pub fn new(buffer: &'a mut Buffer, area: Area) -> Result<Self, DrawErr> {
        let right = area.x() as u32 + area.width() as u32;
        let bottom = area.y() as u32 + area.height() as u32;
        if right > buffer.size().width as u32 || bottom > buffer.size().height as u32 {
            return Err(DrawErr::invalid_area(
                (
                    right.min(u16::MAX as u32) as u16,
                    bottom.min(u16::MAX as u32) as u16,
                )
                    .into(),
                buffer.size(),
            ));
        }

        Ok(Self { buffer, area })
    }

    /// Tests whether local `(x, y)` lies inside this chunk.
    pub fn is_inside(&self, x: u16, y: u16) -> bool {
        self.area.is_inside(x, y)
    }

    /// Writes a grapheme at local `(x, y)` and returns its width in columns.
    ///
    /// Tabs expand to styled spaces using `x` as the tab-stop column. Existing
    /// leading cells can be replaced; only continuation cells are protected.
    ///
    /// # Errors
    ///
    /// Returns [`DrawErr::ContentTooWide`] if the content crosses the right
    /// edge, [`DrawErr::OutOfBounds`] for an invalid position, or
    /// [`DrawErr::PositionOccupied`] when starting in a wide grapheme's tail.
    /// A failed tab expansion can leave earlier spaces already written.
    pub fn set(&mut self, x: u16, y: u16, content: Stylized) -> Result<usize, DrawErr> {
        let content_width = content.width_with(self.text_metrics(), x as usize) as u16;
        if x as usize + content_width as usize > self.area.size().width as usize {
            trace!(
                target: "render::chunk",
                "content too wide at ({}, {}): width={}, available={}",
                x, y, content_width, self.area.size().width.saturating_sub(x)
            );
            return Err(DrawErr::content_too_wide(
                content_width,
                self.area.size().width,
                x,
            ));
        }

        if content.as_char() == Some('\t') {
            for offset in 0..content_width {
                self.set(x + offset, y, Stylized::new(' ', content.style))?;
            }
            return Ok(content_width as usize);
        }

        if let Some(pos) = self.area.to_absolute(x, y) {
            if self.buffer.is_occupied(pos) {
                trace!(
                    target: "render::chunk",
                    "position occupied at ({}, {})",
                    x, y
                );
                return Err(DrawErr::position_occupied(pos));
            }
            self.buffer.set(pos, content)
        } else {
            trace!(
                target: "render::chunk",
                "out of bounds at ({}, {}), area size: {}x{}",
                x, y, self.area.size().width, self.area.size().height
            );
            Err(DrawErr::out_of_bounds((x, y).into(), self.area.size()))
        }
    }

    /// Overwrites local `(x, y)`, merging styles, and returns the display width.
    ///
    /// Replaces any intersecting wide grapheme in full, including portions
    /// outside this chunk. Unspecified style components inherit from the old
    /// destination grapheme. Tabs expand to spaces using the local `x` column.
    ///
    /// # Errors
    ///
    /// Returns [`DrawErr::ContentTooWide`] if content crosses the right edge or
    /// [`DrawErr::OutOfBounds`] if the destination is outside the chunk.
    pub fn set_forced(&mut self, x: u16, y: u16, content: Stylized) -> Result<usize, DrawErr> {
        let content_width = content.width_with(self.text_metrics(), x as usize) as u16;
        if x as usize + content_width as usize > self.area.size().width as usize {
            trace!(
                target: "render::chunk",
                "forced set: content too wide at ({}, {}): width={}, available={}",
                x, y, content_width, self.area.size().width.saturating_sub(x)
            );
            return Err(DrawErr::content_too_wide(
                content_width,
                self.area.size().width,
                x,
            ));
        }

        if content.as_char() == Some('\t') {
            for offset in 0..content_width {
                self.set_forced(x + offset, y, Stylized::new(' ', content.style))?;
            }
            return Ok(content_width as usize);
        }

        if let Some(pos) = self.area.to_absolute(x, y) {
            self.buffer.overwrite(pos, content)
        } else {
            trace!(
                target: "render::chunk",
                "forced set: out of bounds at ({}, {}), area size: {}x{}",
                x, y, self.area.size().width, self.area.size().height
            );
            Err(DrawErr::out_of_bounds((x, y).into(), self.area.size()))
        }
    }

    /// Paints one opaque raster cell at local `(x, y)`.
    ///
    /// Preserves the cell's original target size and offsets, including when
    /// copying a cropped image. Replaces any intersecting wide grapheme in full.
    /// Returns [`DrawErr::OutOfBounds`] for an invalid local position or raster
    /// offset, without modifying the buffer.
    pub fn set_raster_cell(&mut self, x: u16, y: u16, cell: RasterCell) -> Result<(), DrawErr> {
        let pos = self
            .area
            .to_absolute(x, y)
            .ok_or_else(|| DrawErr::out_of_bounds((x, y).into(), self.area.size()))?;
        self.buffer.set_raster(pos, cell)
    }

    /// Copies a raster owner with its already sampled fallback across render/TUI
    /// package boundaries. Internal compositors use this to avoid resampling.
    #[doc(hidden)]
    pub fn set_raster_cell_with_fallback(
        &mut self,
        x: u16,
        y: u16,
        cell: RasterCell,
        fallback: Stylized,
    ) -> Result<(), DrawErr> {
        let pos = self
            .area
            .to_absolute(x, y)
            .ok_or_else(|| DrawErr::out_of_bounds((x, y).into(), self.area.size()))?;
        self.buffer.set_raster_with_fallback(pos, cell, fallback)
    }

    /// Fills this chunk with an opaque raster and its character fallback.
    ///
    /// Each cell retains the full chunk size as its target and its original
    /// local offsets. Empty chunks are a no-op. Canvas clones share pixel data.
    pub fn raster(&mut self, canvas: &RasterCanvas, options: RasterOptions) -> Result<(), DrawErr> {
        let target = self.area.size();
        for y in 0..target.height {
            for x in 0..target.width {
                self.set_raster_cell(
                    x,
                    y,
                    RasterCell {
                        canvas: canvas.clone(),
                        target,
                        x,
                        y,
                        options,
                    },
                )?;
            }
        }
        Ok(())
    }

    /// Marks this frame as containing raster flattened by a visual effect.
    pub fn mark_unsupported_raster_composition(&mut self) {
        self.buffer.mark_unsupported_raster_composition();
    }

    /// Returns the text-width policy shared by the entire underlying buffer.
    pub fn text_metrics(&self) -> crate::render::text::TextMetrics {
        self.buffer.text_metrics()
    }

    /// Read the current render target, including cells outside this chunk.
    ///
    /// Buffer coordinates are absolute. This is useful for sampling previously
    /// painted layers. New glyphs must fit inside the chunk, though replacing
    /// a wide glyph can erase its owner or tail outside the chunk.
    pub fn buffer(&self) -> &Buffer {
        self.buffer
    }

    /// Changes the entire buffer's text-width policy before painting.
    ///
    /// A changed policy clears all buffer content, including outside this
    /// chunk, and discards the previous frame to force a full redraw.
    pub fn set_text_metrics(&mut self, metrics: crate::render::text::TextMetrics) {
        self.buffer.set_text_metrics(metrics);
    }

    /// Paint rich text, preserving graphemes crossing span boundaries.
    ///
    /// Overwrites from local `(x, y)` and stops before a grapheme that would
    /// cross the right edge. Tabs are measured relative to the start of the
    /// supplied line, and zero-width graphemes are skipped.
    ///
    /// # Errors
    ///
    /// Propagates write errors, such as an out-of-bounds row, after any earlier
    /// graphemes have been painted.
    pub fn set_line(
        &mut self,
        x: u16,
        y: u16,
        line: &crate::render::text::Line<'_>,
    ) -> Result<(), DrawErr> {
        let mut column = 0usize;
        for cell in line.cells() {
            let width = cell.width_with(self.text_metrics(), column);
            if x as usize + column + width > self.area.width() as usize {
                break;
            }
            if width == 0 {
                continue;
            }
            if cell.as_char() == Some('\t') {
                for offset in 0..width {
                    self.set_forced(
                        x + (column + offset) as u16,
                        y,
                        Stylized::new(' ', cell.style),
                    )?;
                }
            } else {
                self.set_forced(x + column as u16, y, cell)?;
            }
            column += width;
        }
        Ok(())
    }

    /// Returns this chunk's rectangle in absolute buffer coordinates.
    pub fn area(&self) -> Area {
        self.area
    }

    /// Reborrows this chunk with the specified margins removed.
    ///
    /// Returns [`DrawErr::InvalidShrink`] if the margin totals exceed the
    /// current dimensions. Margin and coordinate sums must fit in `u16`.
    pub fn shrink(
        &mut self,
        top: u16,
        bottom: u16,
        left: u16,
        right: u16,
    ) -> Result<Chunk<'_>, DrawErr> {
        let new_area = self.area.shrink(top, bottom, left, right)?;

        Chunk::new(self.buffer, new_area)
    }

    /// Reborrows the buffer at an absolute area, independently of this chunk.
    ///
    /// The new area need not be contained in the parent chunk. Use
    /// [`with_clip`](Self::with_clip) to enforce that boundary. Returns
    /// [`DrawErr::InvalidArea`] if the area does not fit in the buffer.
    pub fn from_area(&mut self, area: Area) -> Result<Chunk<'_>, DrawErr> {
        Chunk::new(self.buffer, area)
    }

    /// Create a child chunk clipped to this chunk's bounds.
    ///
    /// The supplied area is in absolute buffer coordinates, matching
    /// [`from_area`](Self::from_area). If the area has no overlap with the
    /// current chunk, the closure is not called.
    pub fn with_clip<R>(
        &mut self,
        area: Area,
        render: impl FnOnce(&mut Chunk<'_>) -> R,
    ) -> Result<Option<R>, DrawErr> {
        let Some(clipped) = area.clamp_to(&self.area) else {
            return Ok(None);
        };

        let mut chunk = Chunk::new(self.buffer, clipped)?;
        Ok(Some(render(&mut chunk)))
    }

    /// Overwrites a single line of text from local `(x, y)` with `style`.
    ///
    /// Clips at complete grapheme boundaries on the right. Tabs expand relative
    /// to the start of `string`; control and zero-width graphemes are skipped.
    /// Newlines do not advance to another row; use [`set_text`](Self::set_text)
    /// for multiple lines. Unspecified style components inherit from the buffer.
    ///
    /// # Errors
    ///
    /// Propagates write errors, such as an invalid row. Earlier writes remain.
    pub fn set_string(
        &mut self,
        x: u16,
        y: u16,
        string: &str,
        style: Style,
    ) -> Result<(), DrawErr> {
        use crate::render::text::UnicodeSegmentation;
        let mut column = 0usize;
        let metrics = self.text_metrics();
        for g in string.graphemes(true) {
            let width = metrics.grapheme_width(g, column);
            if x as usize + column + width > self.area.width() as usize {
                break;
            }
            if width == 0 {
                continue;
            }
            if g == "\t" {
                for offset in 0..width {
                    self.set_forced(x + (column + offset) as u16, y, Stylized::new(' ', style))?;
                }
            } else {
                self.set_forced(x + column as u16, y, Stylized::grapheme(g, style))?;
            }
            column += width;
        }
        Ok(())
    }

    /// Paint rich text with explicit line boundaries and clipping.
    ///
    /// Starts at local `(x, y)`, clips each line at the right edge, and stops
    /// at the bottom edge. Propagates errors from [`set_line`](Self::set_line).
    pub fn set_text(
        &mut self,
        x: u16,
        y: u16,
        text: &crate::render::text::Text<'_>,
    ) -> Result<(), DrawErr> {
        for (row, line) in text.lines.iter().enumerate() {
            let row = y as usize + row;
            if row >= self.area.height() as usize {
                break;
            }
            self.set_line(x, row as u16, line)?;
        }
        Ok(())
    }

    /// Overwrites a local rectangle with repetitions of `ch` and `style`.
    ///
    /// Wide characters are repeated only while they fit in the requested
    /// width, possibly leaving a trailing column unchanged. Zero-width
    /// characters paint nothing. Individual write errors are ignored, so
    /// out-of-bounds cells are skipped and this method always returns `Ok(())`.
    pub fn fill(
        &mut self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        ch: char,
        style: Style,
    ) -> Result<(), DrawErr> {
        for dy in 0..height {
            let mut dx = 0usize;
            while dx < width as usize {
                let stylized = Stylized::new(ch, style);
                let columns = stylized.width_with(self.text_metrics(), x as usize + dx);
                if columns == 0 || dx + columns > width as usize {
                    break;
                }
                let _ =
                    self.set_forced(x.saturating_add(dx as u16), y.saturating_add(dy), stylized);
                dx += columns;
            }
        }
        Ok(())
    }

    /// Overwrites one character at local `(x, y)`, merging its style.
    ///
    /// Returns the same bounds and width errors as [`set_forced`](Self::set_forced).
    pub fn set_char(&mut self, x: u16, y: u16, ch: char, style: Style) -> Result<(), DrawErr> {
        let stylized = Stylized::new(ch, style);
        self.set_forced(x, y, stylized)?;
        Ok(())
    }
}

#[cfg(test)]
mod raster_tests {
    use super::*;
    use crate::render::text::{AmbiguousWidth, TextMetrics};

    #[test]
    fn raster_uses_local_offsets_and_chunk_target() {
        let canvas = RasterCanvas::new(2, 2, Default::default()).unwrap();
        let mut buffer = Buffer::new((6, 4).into());
        let area = Area::new((2, 1).into(), (3, 2).into());
        Chunk::new(&mut buffer, area)
            .unwrap()
            .raster(&canvas, RasterOptions::default())
            .unwrap();
        assert_eq!(buffer.raster_cells().count(), 6);
        for (x, y, cell) in buffer.raster_cells() {
            assert_eq!(cell.target, area.size());
            assert_eq!((cell.x, cell.y), (x - 2, y - 1));
            assert_eq!(cell.canvas, canvas);
        }
        assert!(buffer.raster_at((1, 1).into()).is_none());
        assert!(buffer.raster_at((5, 1).into()).is_none());
    }

    #[test]
    fn raster_nested_clip_and_invalid_local_positions() {
        let canvas = RasterCanvas::new(1, 1, Default::default()).unwrap();
        let mut buffer = Buffer::new((5, 3).into());
        let mut chunk = Chunk::new(&mut buffer, Area::new((1, 1).into(), (3, 2).into())).unwrap();
        chunk
            .with_clip(Area::new((2, 0).into(), (3, 2).into()), |clipped| {
                clipped.raster(&canvas, RasterOptions::default()).unwrap();
                let cell = clipped.buffer().raster_at((2, 1).into()).unwrap().clone();
                assert!(clipped.set_raster_cell(2, 0, cell.clone()).is_err());
                assert!(clipped.set_raster_cell(0, 1, cell).is_err());
            })
            .unwrap();
        assert_eq!(buffer.raster_cells().count(), 2);
        let cell = buffer.raster_at((3, 1).into()).unwrap();
        assert_eq!(cell.target, (2, 1).into());
        assert_eq!((cell.x, cell.y), (1, 0));
        assert!(Chunk::new(&mut buffer, Area::new((u16::MAX, 0).into(), (2, 1).into())).is_err());
        Chunk::new(&mut buffer, Area::new((5, 3).into(), (0, 0).into()))
            .unwrap()
            .raster(&canvas, RasterOptions::default())
            .unwrap();
        assert_eq!(buffer.raster_cells().count(), 2);
    }

    #[test]
    fn raster_fallback_stays_one_cell_under_cjk_metrics() {
        let canvas = RasterCanvas::new(2, 2, Default::default()).unwrap();
        let mut buffer = Buffer::new((2, 1).into());
        buffer.set_text_metrics(TextMetrics {
            ambiguous_width: AmbiguousWidth::Wide,
            ..TextMetrics::default()
        });
        Chunk::new(&mut buffer, Area::new((0, 0).into(), (2, 1).into()))
            .unwrap()
            .raster(&canvas, RasterOptions::default())
            .unwrap();
        assert_eq!(buffer.raster_cells().count(), 2);
        assert!(buffer
            .content()
            .iter()
            .all(|cell| !cell.is_occupied && cell.width() == 1));
    }
}
