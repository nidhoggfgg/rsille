use std::slice::Chunks;

use crate::style::Stylized;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct Area {
    pub pos: Position,
    pub size: Size,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Chunk {
    pub size: Size,
    pub content: Vec<Stylized>,
}

impl Chunk {
    pub fn empty(size: (u16, u16)) -> Self {
        Self {
            size: Size {
                width: size.0,
                height: size.1,
            },
            content: vec![Stylized::space(); (size.0 * size.1) as usize],
        }
    }

    pub fn lines(&self) -> Chunks<Stylized> {
        self.content.chunks(self.size.width as usize)
    }

    pub fn index(&self, x: u16, y: u16) -> Option<usize> {
        if self.size.width > x && self.size.height > y {
            Some((y * self.size.width + x) as usize)
        } else {
            None
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Stylized> {
        if let Some(i) = self.index(x, y) {
            Some(&self.content[i])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Stylized> {
        if let Some(i) = self.index(x, y) {
            Some(&mut self.content[i])
        } else {
            None
        }
    }
}
