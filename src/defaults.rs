use crate::{object3d::Object3D, Canvas, Turtle};

// i don't think my Canvas, Objects3D... need a Default trait
// but for making clippy shut up, so add those never used code
impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Object3D {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Turtle {
    fn default() -> Self {
        Self::new()
    }
}
