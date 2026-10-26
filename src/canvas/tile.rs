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
    pub fn get(self) -> (i32, i32) {
        (self.col, self.row)
    }
}
