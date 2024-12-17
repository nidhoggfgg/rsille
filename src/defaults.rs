use crate::{
    extra::{math::Figure, LifeGame, Object3D, Turtle},
    Animation,
};

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

impl Default for LifeGame {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Animation {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Figure {
    fn default() -> Self {
        Self::new()
    }
}
