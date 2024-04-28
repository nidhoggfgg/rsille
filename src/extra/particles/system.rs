use std::time::{Duration, SystemTime};

use glam::Vec3A;

use crate::Paint;

use super::{force::Force, particle::Particle};

pub struct ParticleSystem {
    init_time: SystemTime,
    live_time: Duration,
    paritcles: Vec<Particle>,
    force: Force,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            init_time: SystemTime::now(),
            live_time: Duration::ZERO,
            paritcles: Vec::new(),
            force: Force::new(),
        }
    }

    pub fn with_gravity(mut self, g: f32) -> Self {
        self.force.set_gravity(g);
        self
    }

    pub fn with_drag(mut self, drag: f32) -> Self {
        self.force.set_drag(drag);
        self
    }

    pub fn with_force<F>(mut self, f: F) -> Self
    where
        F: Fn(&Particle) -> Vec3A + Send + 'static,
    {
        self.force.add_force(f);
        self
    }

    pub fn with_particles(mut self, ps: Vec<Particle>) -> Self {
        self.paritcles = ps;
        self
    }

    pub fn add_force<F>(&mut self, f: F)
    where
        F: Fn(&Particle) -> Vec3A + Send + 'static,
    {
        self.force.add_force(f);
    }

    pub fn update(&mut self) -> bool {
        let time = self.init_time.elapsed().unwrap();
        let dt = time - self.live_time;
        for p in &mut self.paritcles {
            p.update(dt, &self.force);
        }
        self.paritcles.retain(|p| !p.is_dead());
        if self.paritcles.is_empty() {
            true
        } else {
            false
        }
    }
}

impl Paint for ParticleSystem {
    fn paint<T>(
        &self,
        canvas: &mut crate::Canvas,
        x: T,
        y: T,
    )
    where
        T: Into<f64>,
    {
        let (x, y) = (x.into(), y.into());
        for p in &self.paritcles {
            canvas.set(x + p.pos.x as f64, y + p.pos.y as f64);
            for t in p.get_trail() {
                canvas.set(x + t.x as f64, y + t.y as f64);
            }
        }
    }
}
