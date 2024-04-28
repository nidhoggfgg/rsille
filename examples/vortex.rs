use std::time::Duration;

use glam::Vec3A;
use rand::Rng;
use rsille::{
    extra::particles::{particle::Particle, system::ParticleSystem},
    Animation,
};

fn main() {
    let ps = gen(100.0, 100);
    let mut anime = Animation::new();
    anime.push(ps, |ps| ps.update(), (0, 0));
    anime.set_fps(15);
    anime.set_minx(-120);
    anime.set_maxy(150);
    anime.run();
}

fn gen(radius: f32, len: usize) -> ParticleSystem {
    let mut p = Vec::new();
    let mut rng = rand::thread_rng();
    while p.len() < len {
        let (vx, vy) = (
            rng.gen_range(-radius..=radius),
            rng.gen_range(-radius..=radius),
        );
        if vx.powi(2) + vx.powi(2) <= radius.powi(2) {
            let vel = Vec3A::new(vy as f32, -vx as f32, 0.0);
            let pos = Vec3A::new(vx as f32, vy as f32, 0.0);
            let p_ = Particle::with(pos, vel, Duration::from_secs_f64(30.0), 100);
            p.push(p_);
        }
    }
    ParticleSystem::new()
        .with_particles(p)
        .with_gravity(0.0)
        .with_drag(0.05)
        .with_force(|p| -p.pos.normalize() * (1.0 / p.pos.length()) * 150.0)
}
