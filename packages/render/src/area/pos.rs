use std::ops::Add;

#[derive(Debug, Clone, Hash, Copy, Default, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub fn right(&mut self, x: u16) {
        self.x += x;
    }

    pub fn down(&mut self, y: u16) {
        self.y += y;
    }

    pub fn reset_x(&mut self) {
        self.x = 0;
    }

    pub fn reset_y(&mut self) {
        self.y = 0;
    }
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

impl Add<Position> for Position {
    type Output = Position;
    fn add(self, rhs: Position) -> Self::Output {
        (self.x + rhs.x, self.y + rhs.y).into()
    }
}
