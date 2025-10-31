//! Common types for widget geometry

/// A rectangle representing a widget's area in terminal cells
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn area(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    pub fn shrink(&self, amount: super::super::style::Padding) -> Rect {
        let new_x = self.x + amount.left;
        let new_y = self.y + amount.top;
        let new_width = self.width.saturating_sub(amount.left + amount.right);
        let new_height = self.height.saturating_sub(amount.top + amount.bottom);
        Rect::new(new_x, new_y, new_width, new_height)
    }
}

/// A 2D size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl Size {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

/// A 2D position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}
