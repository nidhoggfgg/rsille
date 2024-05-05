use std::time::Duration;

use rand::{rngs::ThreadRng, Rng};
use rsille::{
    extra::{math::glm::Vec3, particles::{particle::Particle, system::ParticleSystem}},
    Animation,
};

fn main() {
    let ps = gen(100.0, 100);
    let mut anime = Animation::new();
    anime.push(ps, |ps| ps.update(), (0, 0));
    anime.set_fps(50);
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
            let [r1, r2, r3, r4] = genr4(&mut rng, 10.0);
            let vel = Vec3::new(vy as f32, -vx as f32, r3 + r4);
            let pos = Vec3::new(vx as f32 + r3, vy as f32 + r4, r1 + r2 + r3 + r4);
            let len = rng.gen_range(-100..=100_isize);
            let p_ = Particle::with(pos, vel, Duration::from_secs_f64(50.0), (200 + len) as usize);
            p.push(p_);
        }
    }
    ParticleSystem::new()
        .with_particles(p)
        .with_gravity(0.0)
        .with_drag(0.05)
        .with_force(|p| if p.pos.length() > 1.0 {
            -p.pos.normalize() * (1.0 / p.pos.length()) * 150.0
} else {
    -p.pos.normalize() * p.pos.length()
})
        // .with_force(|p| -p.vel.normalize() * p.vel.length().powi(2) * (1.0 / p.pos.length()))
}

fn genr4(rng: &mut ThreadRng, r: f32) -> [f32; 4] {
    [rng.gen_range(-r..=r),
     rng.gen_range(-r..=r),
     rng.gen_range(-r..=r),
     rng.gen_range(-r..=r)]
}