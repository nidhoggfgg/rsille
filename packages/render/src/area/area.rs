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

    // Convenience accessors for compatibility with flat-field access patterns
    pub fn x(&self) -> u16 {
        self.pos.x
    }

    pub fn y(&self) -> u16 {
        self.pos.y
    }

    pub fn width(&self) -> u16 {
        self.size.width
    }

    pub fn height(&self) -> u16 {
        self.size.height
    }

    /// Returns the total area in cells (width * height)
    pub fn area(&self) -> usize {
        (self.size.width as usize) * (self.size.height as usize)
    }

    /// Check if this area intersects with another
    pub fn intersects(&self, other: &Area) -> bool {
        self.pos.x < other.pos.x + other.size.width
            && self.pos.x + self.size.width > other.pos.x
            && self.pos.y < other.pos.y + other.size.height
            && self.pos.y + self.size.height > other.pos.y
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
        let shrink_total = (left + right, bottom + top).into();
        if self.size.less_any(shrink_total) {
            return Err(DrawErr::invalid_shrink(self.size, shrink_total));
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

    /// Shrink the area with saturating subtraction (never returns an error)
    pub fn shrink_saturating(self, top: u16, bottom: u16, left: u16, right: u16) -> Self {
        let new_x = self.pos.x + left;
        let new_y = self.pos.y + top;
        let new_width = self.size.width.saturating_sub(left + right);
        let new_height = self.size.height.saturating_sub(top + bottom);

        Self {
            pos: (new_x, new_y).into(),
            size: (new_width, new_height).into(),
        }
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
