use crate::{
    area::{Position, Size},
    DrawErr,
};

#[derive(Debug, Clone, Hash, Copy, Default, PartialEq, Eq)]
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

    /// Check if this area completely contains another area
    pub fn contains_area(&self, other: &Area) -> bool {
        let self_right = self.pos.x.saturating_add(self.size.width);
        let self_bottom = self.pos.y.saturating_add(self.size.height);
        let other_right = other.pos.x.saturating_add(other.size.width);
        let other_bottom = other.pos.y.saturating_add(other.size.height);

        other.pos.x >= self.pos.x
            && other.pos.y >= self.pos.y
            && other_right <= self_right
            && other_bottom <= self_bottom
    }

    /// Check if this area intersects with another area
    pub fn intersects(&self, other: &Area) -> bool {
        let self_right = self.pos.x.saturating_add(self.size.width);
        let self_bottom = self.pos.y.saturating_add(self.size.height);
        let other_right = other.pos.x.saturating_add(other.size.width);
        let other_bottom = other.pos.y.saturating_add(other.size.height);

        !(self_right <= other.pos.x
            || other_right <= self.pos.x
            || self_bottom <= other.pos.y
            || other_bottom <= self.pos.y)
    }

    /// Clamp this area to fit within bounds, returning None if there's no overlap
    pub fn clamp_to(&self, bounds: &Area) -> Option<Area> {
        if !self.intersects(bounds) {
            return None;
        }

        let bounds_right = bounds.pos.x.saturating_add(bounds.size.width);
        let bounds_bottom = bounds.pos.y.saturating_add(bounds.size.height);
        let self_right = self.pos.x.saturating_add(self.size.width);
        let self_bottom = self.pos.y.saturating_add(self.size.height);

        let new_x = self.pos.x.max(bounds.pos.x);
        let new_y = self.pos.y.max(bounds.pos.y);
        let new_right = self_right.min(bounds_right);
        let new_bottom = self_bottom.min(bounds_bottom);

        let new_width = new_right.saturating_sub(new_x);
        let new_height = new_bottom.saturating_sub(new_y);

        if new_width == 0 || new_height == 0 {
            return None;
        }

        Some(Area::new(
            (new_x, new_y).into(),
            (new_width, new_height).into(),
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_area() {
        let outer = Area::new((10, 10).into(), (20, 20).into());
        let inner = Area::new((15, 15).into(), (5, 5).into());
        let partial = Area::new((25, 25).into(), (10, 10).into());
        let outside = Area::new((50, 50).into(), (5, 5).into());

        assert!(
            outer.contains_area(&inner),
            "Inner area should be contained"
        );
        assert!(
            !outer.contains_area(&partial),
            "Partially overlapping area should not be contained"
        );
        assert!(
            !outer.contains_area(&outside),
            "Outside area should not be contained"
        );
    }

    #[test]
    fn test_intersects() {
        let area1 = Area::new((10, 10).into(), (20, 20).into());
        let area2 = Area::new((20, 20).into(), (20, 20).into());
        let area3 = Area::new((50, 50).into(), (10, 10).into());

        assert!(
            area1.intersects(&area2),
            "Overlapping areas should intersect"
        );
        assert!(
            !area1.intersects(&area3),
            "Non-overlapping areas should not intersect"
        );
    }

    #[test]
    fn test_clamp_to() {
        let bounds = Area::new((10, 10).into(), (20, 20).into());

        // Area completely inside bounds
        let inside = Area::new((15, 15).into(), (5, 5).into());
        let clamped = inside.clamp_to(&bounds);
        assert_eq!(
            clamped,
            Some(inside),
            "Area inside bounds should remain unchanged"
        );

        // Area partially outside bounds
        let partial = Area::new((25, 25).into(), (10, 10).into());
        let clamped = partial.clamp_to(&bounds);
        assert!(
            clamped.is_some(),
            "Partially overlapping area should be clamped"
        );
        let clamped_area = clamped.unwrap();
        assert_eq!(clamped_area.x(), 25);
        assert_eq!(clamped_area.y(), 25);
        assert_eq!(clamped_area.width(), 5); // Clamped from 10 to 5
        assert_eq!(clamped_area.height(), 5); // Clamped from 10 to 5

        // Area completely outside bounds
        let outside = Area::new((50, 50).into(), (10, 10).into());
        let clamped = outside.clamp_to(&bounds);
        assert_eq!(clamped, None, "Area outside bounds should return None");
    }
}
