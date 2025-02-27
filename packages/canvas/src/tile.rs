use crate::utils::get_pos;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub(crate) struct Tile {
    col: i32,
    row: i32,
}

impl Tile {
    #[inline]
    #[must_use]
    pub fn from(col: i32, row: i32) -> Self {
        Self { col, row }
    }

    #[inline]
    #[must_use]
    pub fn from_xy<T>(x: T, y: T) -> Self
    where
        T: Into<f64> + Copy,
    {
        let (col, row) = get_pos(x, y);
        Self { col, row }
    }

    #[inline]
    #[must_use]
    pub fn get(self) -> (i32, i32) {
        (self.col, self.row)
    }
}
