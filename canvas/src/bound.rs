#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct Bound {
    minx: i32,
    miny: i32,
    maxx: i32,
    maxy: i32,
}

impl Bound {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_inside(&self, x: i32, y: i32) -> bool {
        self.minx <= x && x <= self.maxx && self.miny <= y && y <= self.maxy
    }

    pub fn update(&mut self, x: i32, y: i32) {
        if !self.is_inside(x, y) {
            self.minx = self.minx.min(x);
            self.maxx = self.maxx.max(x);
            self.miny = self.miny.min(y);
            self.maxy = self.maxy.max(y);
        }
    }

    pub fn get_bound(&self) -> ((i32, i32), (i32, i32)) {
        ((self.minx, self.maxx), (self.miny, self.maxy))
    }
}
