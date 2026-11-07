//! Inclusive terminal-cell bounds and conversion to terminal coordinates.

use crate::canvas::tile::Tile;

/// A Cartesian cell rectangle, optionally frozen against automatic expansion.
///
/// The default rectangle contains only `(0, 0)` and is not fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct Bound {
    minx: i32,
    miny: i32,
    maxx: i32,
    maxy: i32,
    fixed: bool,
}

impl Bound {
    /// Creates unfixed bounds containing only the origin cell.
    #[inline]
    #[must_use]
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Tests whether a cell coordinate lies within the inclusive rectangle.
    #[inline]
    #[must_use]
    pub(crate) fn is_inside(&self, x: i32, y: i32) -> bool {
        self.minx <= x && x <= self.maxx && self.miny <= y && y <= self.maxy
    }

    /// Tests whether `tile` lies within the inclusive rectangle.
    #[inline]
    #[must_use]
    pub(crate) fn tile_inside(&self, tile: Tile) -> bool {
        let (x, y) = tile.get();
        self.is_inside(x, y)
    }

    /// Converts a tile to an offset from the rectangle's top-left corner.
    ///
    /// Reverses the y-axis for terminal output. Returns `None` for an excluded
    /// tile or an offset that would overflow `i32`.
    #[inline]
    #[must_use]
    pub(crate) fn get_terminal_xy(&self, tile: Tile) -> Option<(i32, i32)> {
        if !self.tile_inside(tile) {
            return None;
        }

        let (x, y) = tile.get();
        // reverse the y-axis, because the y-axis is reversed in the terminal
        Some((x.checked_sub(self.minx)?, self.maxy.checked_sub(y)?))
    }

    /// Expands the rectangle to include `tile`, unless the bounds are fixed.
    #[inline]
    pub(crate) fn update(&mut self, tile: Tile) {
        if self.fixed {
            return;
        }
        let (x, y) = tile.get();
        if !self.is_inside(x, y) {
            self.minx = self.minx.min(x);
            self.maxx = self.maxx.max(x);
            self.miny = self.miny.min(y);
            self.maxy = self.maxy.max(y);
        }
    }

    /// Replaces both inclusive ranges without changing the fixed flag.
    ///
    /// The caller must ensure that each minimum is at most its maximum.
    #[inline]
    pub(crate) fn set_bound(&mut self, range_x: (i32, i32), range_y: (i32, i32)) {
        (self.minx, self.maxx) = range_x;
        (self.miny, self.maxy) = range_y;
    }

    /// Returns `((min_x, max_x), (min_y, max_y))` in terminal cells.
    #[inline]
    pub(crate) fn get_bound(&self) -> ((i32, i32), (i32, i32)) {
        ((self.minx, self.maxx), (self.miny, self.maxy))
    }

    /// Enables or disables automatic-expansion prevention.
    #[inline]
    pub(crate) fn fixed_bound(&mut self, is_fixed: bool) {
        self.fixed = is_fixed;
    }

    /// Returns whether automatic expansion is disabled.
    pub(crate) fn is_fixed(&self) -> bool {
        self.fixed
    }
}
