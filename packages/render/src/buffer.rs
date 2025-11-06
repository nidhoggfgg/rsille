use crate::{
    DrawErr,
    area::{Position, Size},
    style::Stylized,
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
        let i = self.index_unchecked(pos);
        if i < self.content.len() {
            self.content[i].is_occupied
        } else {
            false
        }
    }

    pub fn get(&self, pos: Position) -> Option<&Stylized> {
        let i = self.index_unchecked(pos);
        if i < self.content.len() {
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
        let i = self.index_unchecked(pos);
        if i >= self.content.len() {
            return Err(DrawErr::out_of_bounds(pos, self.size));
        }

        if self.content[i].is_occupied {
            return Err(DrawErr::position_occupied(pos));
        }

        self.set_forced(pos, content)
    }

    pub fn set_forced(&mut self, pos: Position, content: Stylized) -> Result<usize, DrawErr> {
        let i = self.index_unchecked(pos);
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
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn content(&self) -> &[Cell] {
        &self.content
    }

    fn index_unchecked(&self, pos: Position) -> usize {
        (pos.y * self.size.width + pos.x) as usize
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

        self.size = new_size;
        self.content = vec![Cell::space(); (new_size.width * new_size.height) as usize];
        // Clear previous buffer on resize to force full redraw
        self.previous = None;
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
        let mut buffer = Buffer::new(Size { width: 10, height: 10 });

        // Initial size should be 10x10
        assert_eq!(buffer.size(), Size { width: 10, height: 10 });
        assert_eq!(buffer.content.len(), 100);

        // Resize to 20x15
        buffer.resize(Size { width: 20, height: 15 });

        // Check new size
        assert_eq!(buffer.size(), Size { width: 20, height: 15 });
        assert_eq!(buffer.content.len(), 300);

        // Previous buffer should be cleared on resize
        assert!(buffer.previous().is_none());
    }

    #[test]
    fn test_buffer_resize_same_size() {
        let mut buffer = Buffer::new(Size { width: 10, height: 10 });
        buffer.clear(); // Create a previous buffer

        assert!(buffer.previous().is_some());

        // Resize to same size should do nothing
        buffer.resize(Size { width: 10, height: 10 });

        // Previous buffer should still exist
        assert!(buffer.previous().is_some());
    }
}
