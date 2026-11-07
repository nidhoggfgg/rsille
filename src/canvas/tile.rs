//! Hashable cell coordinates for sparse canvas storage.

/// A terminal cell's column and Cartesian row, independent of sample density.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub(crate) struct Tile {
    col: i32,
    row: i32,
}

impl Tile {
    /// Creates a tile at `(col, row)`, with rows increasing upwards.
    #[inline]
    #[must_use]
    pub fn from(col: i32, row: i32) -> Self {
        Self { col, row }
    }

    /// Returns the cell's `(column, row)` coordinates.
    #[inline]
    #[must_use]
    pub fn get(self) -> (i32, i32) {
        (self.col, self.row)
    }
}
