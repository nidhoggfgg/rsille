use std::ops::Add;

use crate::area::Size;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
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

impl Add<Size> for Position {
    type Output = Position;

    fn add(self, rhs: Size) -> Self::Output {
        Self {
            x: self.x + rhs.width,
            y: self.y + rhs.height,
        }
    }
}

impl Add<Position> for Position {
    type Output = Position;

    fn add(self, rhs: Position) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
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
