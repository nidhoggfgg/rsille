use crate::{
    DrawErr,
    area::{Position, Size},
};

#[derive(Debug, Clone, Hash, Copy, Default)]
pub struct Area {
    pos: Position,
    size: Size,
}

// x, y in Area is relative to the postion
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

        Some((x + self.pos.x, y + self.pos.y).into())
    }

    pub fn is_inside(&self, x: u16, y: u16) -> bool {
        x < self.size.width && y < self.size.height
    }

    pub fn shrink(self, top: u16, bottom: u16, left: u16, right: u16) -> Result<Self, DrawErr> {
        if self.size.less_any((left + right, bottom + top).into()) {
            return Err(DrawErr);
        }

        Ok(Self {
            pos: (self.pos.x + left, self.pos.y + top).into(),
            size: (
                self.size.width - left - right,
                self.size.height - bottom - top,
            )
                .into(),
        })
    }

    pub fn real_size(&self) -> Size {
        (self.size.width + self.pos.x, self.size.height + self.pos.y).into()
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
