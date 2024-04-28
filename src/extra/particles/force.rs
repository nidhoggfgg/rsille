use std::time::Duration;

use glam::Vec3A;

use super::particle::Particle;

pub struct Force {
    gravity: f32,
    drag: f32,
    extra: Vec<Box<dyn Fn(&Particle) -> Vec3A + Send + 'static>>,
    dt: f32,
}

impl Force {
    pub fn new() -> Self {
        Self {
            gravity: 1.0,
            drag: 0.0,
            extra: Vec::new(),
            dt: 0.001,
        }
    }

    pub fn with(gravity: f32, drag: f32, dt: f32) -> Self {
        Self {
            gravity,
            drag,
            extra: Vec::new(),
            dt,
        }
    }

    pub fn with_gravity(mut self, g: f32) -> Self {
        self.gravity = g;
        self
    }

    pub fn with_air(mut self, drag: f32) -> Self {
        self.drag = drag;
        self
    }

    pub fn with_force<F>(mut self, f: F) -> Self
    where
        F: Fn(&Particle) -> Vec3A + Send + 'static,
    {
        self.extra.push(Box::new(f));
        self
    }

    pub fn add_force<F>(&mut self, f: F)
    where
        F: Fn(&Particle) -> Vec3A + Send + 'static,
    {
        self.extra.push(Box::new(f));
    }

    pub fn set_gravity(&mut self, g: f32) {
        self.gravity = g;
    }

    pub fn set_drag(&mut self, drag: f32) {
        self.drag = drag;
    }

    pub fn apply(&self, p: &mut Particle, time: Duration) {
        let (mut vel, mut pos) = (p.vel, p.pos);
        let mut t = 0.0;
        let time = time.as_secs_f32();
        while t < time {
            vel += self.dt
                * (Vec3A::NEG_Y * 9.8 * self.gravity
                    - vel.normalize() * vel.length().powi(2) * self.drag);
            for f in &self.extra {
                vel += self.dt * (f)(p);
            }
            pos += self.dt * vel;
            t += self.dt;
        }
        p.vel = vel;
        p.pos = pos;
    }
}
