use crate::DrawErr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl From<(u16, u16)> for Size {
    fn from((width, height): (u16, u16)) -> Self {
        Self { width, height }
    }
}

impl From<Size> for (u16, u16) {
    fn from(size: Size) -> Self {
        (size.width, size.height)
    }
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

impl From<Position> for (u16, u16) {
    fn from(pos: Position) -> Self {
        (pos.x, pos.y)
    }
}

impl From<(u16, u16)> for Position {
    fn from((x, y): (u16, u16)) -> Self {
        Self { x, y }
    }
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
        let (max_width, max_height) = self.size.into();
        let (pos_x, pos_y) = self.pos.into();

        if pos_x + left + right > max_width || pos_y + top + bottom > max_height {
            return Err(DrawErr);
        }

        Ok(Self {
            pos: Position {
                x: pos_x + left,
                y: pos_y + top,
            },
            size: Size {
                width: max_width - right,
                height: max_height - bottom,
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
