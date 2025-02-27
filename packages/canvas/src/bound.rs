use crate::tile::Tile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct Bound {
    minx: i32,
    miny: i32,
    maxx: i32,
    maxy: i32,
    fixed: bool,
}

impl Bound {
    #[inline]
    #[must_use]
    pub(crate) fn new() -> Self {
        Default::default()
    }

    #[inline]
    #[must_use]
    pub(crate) fn is_inside(&self, x: i32, y: i32) -> bool {
        self.minx <= x && x <= self.maxx && self.miny <= y && y <= self.maxy
    }

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

    #[inline]
    pub(crate) fn set_bound(&mut self, range_x: (i32, i32), range_y: (i32, i32)) {
        (self.minx, self.maxx) = range_x;
        (self.miny, self.maxy) = range_y;
    }

    #[inline]
    pub(crate) fn get_bound(&self) -> ((i32, i32), (i32, i32)) {
        ((self.minx, self.maxx), (self.miny, self.maxy))
    }

    #[inline]
    pub(crate) fn fixed_bound(&mut self, is_fixed: bool) {
        self.fixed = is_fixed;
    }
}
