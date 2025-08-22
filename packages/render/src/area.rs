use crate::DrawErr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct MayBeSize {
    pub width: Option<u16>,
    pub height: Option<u16>,
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

impl Area {
    pub fn new(pos: Position, size: Size) -> Self {
        Self { pos, size }
    }

    pub fn shrink(self, size: u16) -> Result<Self, DrawErr> {
        if self.size.width < size || self.size.height < size {
            return Err(DrawErr);
        }

        if self.pos.x + size > self.size.width - size || self.pos.y + size > self.size.height - size
        {
            return Err(DrawErr);
        }

        let new_pos = Position {
            x: self.pos.x + size,
            y: self.pos.y + size,
        };
        let new_size = Size {
            width: self.size.width - size,
            height: self.size.height - size,
        };

        Ok(Self {
            pos: new_pos,
            size: new_size,
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
