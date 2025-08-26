use crate::{
    DrawErr,
    area::{Position, Size},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct Area {
    pos: Position,
    size: Size,
}

impl Area {
    pub fn new(pos: Position, size: Size) -> Self {
        Self { pos, size }
    }

    pub fn pos(&self) -> Position {
        self.pos
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn to_absolute(&self, x: u16, y: u16) -> Option<Position> {
        if !self.is_inside(x, y) {
            return None;
        }

        Some(Position {
            x: x + self.pos.x,
            y: y + self.pos.y,
        })
    }

    pub fn is_inside(&self, x: u16, y: u16) -> bool {
        x < self.size.width && y < self.size.height
    }

    pub fn shrink(self, top: u16, bottom: u16, left: u16, right: u16) -> Result<Self, DrawErr> {
        if self.size < (left + right, top + bottom) {
            return Err(DrawErr);
        }

        Ok(Self {
            pos: Position {
                x: self.pos.x + left,
                y: self.pos.y + top,
            },
            size: Size {
                width: self.size.width - right,
                height: self.size.height - bottom,
            },
        })
    }
}

impl From<Size> for Area {
    fn from(size: Size) -> Self {
        Self::new(Position::default(), size)
    }
}

impl From<Position> for Area {
    fn from(pos: Position) -> Self {
        Self::new(pos, Size::default())
    }
}
