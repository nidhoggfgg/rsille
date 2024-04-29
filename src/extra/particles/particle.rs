use std::{collections::VecDeque, time::Duration};

use crate::extra::math::na::Vec3;

use super::force::Force;

#[derive(Debug, Clone)]
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub mass: f64,

    life_time: Duration,
    alive_time: Duration,
    is_dead: bool,
    early_dead: bool,
    trail_len: usize,
    trail: VecDeque<Vec3>,
}

impl Particle {
    pub fn new() -> Self {
        Self {
            pos: Vec3::zeros(),
            vel: Vec3::zeros(),
            mass: 1.0,
            life_time: Duration::ZERO,
            alive_time: Duration::ZERO,
            is_dead: false,
            early_dead: false,
            trail_len: 0,
            trail: VecDeque::new(),
        }
    }

    pub fn with_pos(mut self, pos: Vec3) -> Self {
        self.pos = pos;
        self
    }

    pub fn with_vel(mut self, vel: Vec3) -> Self {
        self.vel = vel;
        self
    }

    pub fn with_life(mut self, life_time: Duration) -> Self {
        self.life_time = life_time;
        self
    }

    pub fn with_trail_len(mut self, trail_len: usize) -> Self {
        self.trail_len = trail_len;
        self
    }

    pub fn with(pos: Vec3, vel: Vec3, life_time: Duration, trail_len: usize) -> Self {
        Self {
            pos,
            vel,
            life_time,
            trail_len,
            ..Default::default()
        }
    }

    pub fn update(&mut self, dt: Duration, force: &Force) -> bool {
        if self.early_dead && self.is_dead {
            self.trail = VecDeque::new();
            return true;
        }

        if !self.is_dead {
            self.alive_time += dt;
            if self.alive_time >= self.life_time {
                self.is_dead = true;
            }
            force.apply(self, dt);
            self.trail.push_back(self.pos);
        }

        if self.trail.len() > self.trail_len || self.is_dead {
            self.trail.pop_front();
        }

        return false;
    }

    pub fn is_dead(&self) -> bool {
        if self.early_dead {
            self.is_dead
        } else {
            self.trail.is_empty()
        }
    }

    pub fn get_trail(&self) -> &VecDeque<Vec3> {
        &self.trail
    }
}
