use log::info;

use crate::{
    area::{Position, Size},
    style::Stylized,
    DrawErr,
};

#[derive(Debug, Clone, Default)]
pub struct Buffer {
    size: Size,
    content: Vec<Cell>,
    previous: Option<Vec<Cell>>,
}

// every position in the buffer is absolute
impl Buffer {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            content: vec![Cell::space(); (size.width * size.height) as usize],
            previous: None,
        }
    }

    pub fn index(&self, pos: Position) -> Option<usize> {
        if self.size.less_any((pos.x, pos.y).into()) {
            return None;
        }

        Some((pos.y * self.size.width + pos.x) as usize)
    }

    pub fn is_occupied(&self, pos: Position) -> bool {
        if let Some(i) = self.index(pos) {
            self.content[i].is_occupied
        } else {
            false
        }
    }

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

    pub fn set(&mut self, pos: Position, content: Stylized) -> Result<usize, DrawErr> {
        if let Some(i) = self.index(pos) {
            let width = content.width();
            if i + width > self.content.len() || i == self.content.len() {
                return Err(DrawErr::out_of_bounds(pos, self.size));
            }
            if self.content[i].is_occupied {
                return Err(DrawErr::position_occupied(pos));
            }
            self.content[i].content = content;
            for j in 1..width {
                self.content[i + j].is_occupied = true;
            }
            Ok(width)
        } else {
            Err(DrawErr::out_of_bounds(pos, self.size))
        }
    }

    pub fn overwrite(&mut self, pos: Position, content: Stylized) -> Result<usize, DrawErr> {
        if let Some(i) = self.index(pos) {
            let width = content.width();
            if i + width > self.content.len() || i == self.content.len() {
                return Err(DrawErr::out_of_bounds(pos, self.size));
            }

            if self.content[i].is_occupied {
                if let Some(owner) = self.content[i].owner {
                    for j in owner..i {
                        self.content[j] = Cell::space();
                    }
                }
            }

            self.content[i] = Cell::new(content);
            for j in i + 1..i + width {
                self.content[j].is_occupied = true;
                self.content[j].owner = Some(i);
            }
            Ok(width)
        } else {
            Err(DrawErr::out_of_bounds(pos, self.size))
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn content(&self) -> &[Cell] {
        &self.content
    }

    /// Clear the buffer for the next frame, preserving previous for diffing
    pub fn clear(&mut self) {
        // Save current content as previous
        self.previous = Some(self.content.clone());
        // Reset all cells to empty
        self.content.fill(Cell::space());
    }

    /// Reset diff state (clears previous buffer)
    pub fn reset_diff(&mut self) {
        self.previous = None;
    }

    /// Get the previous buffer state for diffing
    pub fn previous(&self) -> Option<&[Cell]> {
        self.previous.as_deref()
    }

    /// Resize the buffer to a new size
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
        self.content = vec![Cell::space(); (new_size.width * new_size.height) as usize];
        // Clear previous buffer on resize to force full redraw
        self.previous = None;
    }

    /// Returns an iterator over cells that changed since the previous frame
    pub fn diff(&self) -> Option<DiffIterator<'_>> {
        self.previous.as_ref().map(|prev| DiffIterator {
            current: &self.content,
            previous: prev,
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
            width: self.size.width,
            height: self.size.height,
            line_index: 0,
        }
    }
}

/// Iterator over changed cells between current and previous frame
pub struct DiffIterator<'a> {
    current: &'a [Cell],
    previous: &'a [Cell],
    index: usize,
    width: u16,
}

impl<'a> Iterator for DiffIterator<'a> {
    type Item = (u16, u16, &'a Cell);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.current.len() {
            let i = self.index;
            self.index += 1;

            let curr = &self.current[i];
            let prev = &self.previous[i];

            // Only yield cells that changed and are not occupied by wide characters
            if curr != prev && !curr.is_occupied {
                let x = (i as u16) % self.width;
                let y = (i as u16) / self.width;
                return Some((x, y, curr));
            }
        }
        None
    }
}

/// Iterator over all visible cells in the buffer
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
                let x = (i as u16) % self.width;
                let y = (i as u16) / self.width;
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
            if current_line == prev_line {
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cell {
    pub content: Stylized,
    pub is_occupied: bool,
    pub owner: Option<usize>,
}

impl Cell {
    pub fn raw(c: char) -> Self {
        Self {
            content: Stylized::plain(c),
            is_occupied: false,
            owner: None,
        }
    }

    pub fn new(content: Stylized) -> Self {
        Self {
            content,
            is_occupied: false,
            owner: None,
        }
    }

    pub fn space() -> Self {
        Self {
            content: Stylized::space(),
            is_occupied: false,
            owner: None,
        }
    }

    pub fn width(&self) -> usize {
        self.content.width()
    }

    pub fn width_cjk(&self) -> usize {
        self.content.width_cjk()
    }

    pub fn queue(&self, buffer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.content.queue(buffer)
    }

    pub fn has_color(&self) -> bool {
        self.content.has_color()
    }

    pub fn has_attr(&self) -> bool {
        self.content.has_attr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .any(|(x, y, cell)| { *x == 1 && *y == 0 && cell.content.c == Some('X') }));

        // Check new cell at (3, 0)
        assert!(diff
            .iter()
            .any(|(x, y, cell)| { *x == 3 && *y == 0 && cell.content.c == Some('d') }));
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
            .any(|(x, y, cell)| { *x == 0 && *y == 0 && cell.content.c == Some('a') }));
        assert!(all
            .iter()
            .any(|(x, y, cell)| { *x == 1 && *y == 0 && cell.content.c == Some('b') }));
        assert!(all
            .iter()
            .any(|(x, y, cell)| { *x == 0 && *y == 1 && cell.content.c == Some('c') }));
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
}
