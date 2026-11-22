//! Terminal-cell positions and relative movement.

use std::ops::Add;

/// A zero-based column and row, with `(0, 0)` at the top-left corner.
///
/// Movement and addition use ordinary `u16` arithmetic. Callers must keep
/// coordinates in range; overflow or underflow panics when overflow checks are
/// enabled. The default position is the origin.
#[derive(Debug, Clone, Hash, Copy, Default, PartialEq, Eq)]
pub struct Position {
    /// Column, increasing to the right.
    pub x: u16,
    /// Row, increasing downward.
    pub y: u16,
}

impl Position {
    /// Moves right by `x` columns.
    pub fn right(&mut self, x: u16) {
        self.x += x;
    }

    /// Moves left by `x` columns.
    pub fn left(&mut self, x: u16) {
        self.x -= x;
    }

    /// Moves up by `y` rows.
    pub fn up(&mut self, y: u16) {
        self.y -= y;
    }

    /// Moves down by `y` rows.
    pub fn down(&mut self, y: u16) {
        self.y += y;
    }

    /// Resets both coordinates to zero.
    pub fn reset(&mut self) {
        self.x = 0;
        self.y = 0;
    }

    /// Resets the column to zero, preserving the row.
    pub fn reset_x(&mut self) {
        self.x = 0;
    }

    /// Resets the row to zero, preserving the column.
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
