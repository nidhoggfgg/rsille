use glam::Vec3A;

use crate::{
    extra::{
        math::Figure,
        particles::{
            force::Force,
            particle::{self, Particle},
            system::ParticleSystem,
        },
        LifeGame, Object3D, Turtle,
    },
    Animation, Canvas,
};

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

impl Default for Particle {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Force {
    fn default() -> Self {
        Self::new()
    }
}
