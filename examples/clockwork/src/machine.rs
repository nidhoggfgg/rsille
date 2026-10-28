//! The same physical inventory assembles into three articulated machines.
use crate::math::{random, smooth, V3};
use std::f32::consts::{PI, TAU};

pub const PARTS: usize = 132;
pub const PERIOD: f64 = 18.0;
pub const STEP: f64 = 1.0 / 120.0;
pub const NAMES: [&str; 3] = ["STRIDER", "ORNITHOPTER", "CLOCKWORK IRIS"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Brass,
    Porcelain,
    Steel,
    Light,
}

#[derive(Clone, Copy)]
pub struct Pose {
    pub center: V3,
    /// The local Y axis: a beam's length or a cog's axle.
    pub axis: V3,
    pub size: V3,
}

#[derive(Clone)]
pub struct Part {
    pub pose: Pose,
    pub velocity: V3,
    pub material: Material,
    pub gear: bool,
    pub spin: f32,
}

pub struct Machine {
    pub parts: Vec<Part>,
    pub time: f64,
    pub age: f64,
    pub form: usize,
    pub autoplay: bool,
    pub exploded: bool,
    pub pulse: f32,
    remainder: f64,
}

impl Default for Machine {
    fn default() -> Self {
        let parts = (0..PARTS)
            .map(|i| Part {
                pose: Pose {
                    center: staging(i),
                    axis: V3::Y,
                    size: V3(0.1, 0.1, 0.1),
                },
                velocity: V3::ZERO,
                material: material(i),
                gear: if i < 12 {
                    matches!(i, 0 | 1 | 6..=11)
                } else {
                    matches!((i - 12) % 10, 4..=6)
                },
                spin: random(i + 93) * TAU,
            })
            .collect();
        Self {
            parts,
            time: 0.0,
            age: 0.0,
            form: 0,
            autoplay: true,
            exploded: false,
            pulse: 0.0,
            remainder: 0.0,
        }
    }
}

impl Machine {
    pub fn select(&mut self, form: usize) {
        self.form = form % 3;
        self.age = 0.0;
        self.autoplay = false;
    }

    pub fn kick(&mut self) {
        for (i, part) in self.parts.iter_mut().enumerate() {
            let radial = V3(part.pose.center.0, 0.0, part.pose.center.2).unit();
            part.velocity =
                radial * (2.5 + random(i + 19) * 3.0) + V3::Y * (2.0 + random(i + 37) * 4.0);
        }
        self.pulse = 1.2;
    }

    /// A fixed step makes playback independent of render rate and snapshots reproducible.
    pub fn advance(&mut self, delta: f64) {
        self.remainder += delta;
        while self.remainder + 1e-10 >= STEP {
            self.tick();
            self.remainder -= STEP;
        }
    }

    fn tick(&mut self) {
        let dt = STEP as f32;
        self.time += STEP;
        self.age += STEP;
        if self.autoplay && self.age + 1e-8 >= PERIOD {
            self.age -= PERIOD;
            self.form = (self.form + 1) % 3;
        }
        self.pulse = (self.pulse - dt).max(0.0);
        let targets = targets(self.form, self.time as f32);
        for (i, (part, pose)) in self.parts.iter_mut().zip(targets).enumerate() {
            let delay = ((i.saturating_sub(12) / 10) as f32 * 0.08 + random(i + 7) * 0.3).min(1.2);
            let build = smooth((self.age as f32 - delay) / 2.4);
            let release = if self.autoplay {
                smooth((self.age as f32 - 14.5 - delay * 0.3) / 2.2)
            } else {
                0.0
            };
            let dock = build * (1.0 - release);
            let mut assembled = pose.center;
            if self.exploded {
                let center = V3(0.0, 1.7, 0.0);
                assembled =
                    center + (assembled - center) * 1.38 + (assembled - center).unit() * 0.35;
            }
            let mut goal = staging(i).lerp(assembled, dock);
            goal.1 += (dock * PI).sin() * (0.7 + random(i + 21) * 0.8);
            // A pulse briefly disables magnetic docking, letting the parts tumble ballistically.
            let spring = if self.pulse > 0.55 { 0.0 } else { 52.0 };
            let damping = if spring == 0.0 { 0.7 } else { 11.0 };
            let force = (goal - part.pose.center) * spring - part.velocity * damping;
            part.velocity =
                part.velocity + (force - V3::Y * if spring == 0.0 { 5.0 } else { 0.0 }) * dt;
            part.pose.center = part.pose.center + part.velocity * dt;
            let floor = 0.11;
            if part.pose.center.1 < floor {
                part.pose.center.1 = floor;
                part.velocity.1 = part.velocity.1.abs() * 0.25;
                part.velocity.0 *= 0.85;
                part.velocity.2 *= 0.85;
            }
            let axis = V3::Y.lerp(pose.axis, dock).unit();
            part.pose.axis = part.pose.axis.lerp(axis, dt * 9.0).unit();
            part.pose.size = part.pose.size.lerp(pose.size, dt * 8.0);
            part.spin += dt * (if i % 2 == 0 { 1.0 } else { -1.0 }) * (0.4 + dock * 2.0);
        }
    }

    pub fn phase(&self) -> &'static str {
        if self.pulse > 0.1 {
            "MAGNETIC PULSE"
        } else if self.exploded {
            "EXPLODED VIEW"
        } else if self.age < 4.5 {
            "ASSEMBLING"
        } else if self.autoplay && self.age > 14.5 {
            "UNDOCKING"
        } else {
            "ALIVE"
        }
    }
}

fn material(i: usize) -> Material {
    if i < 12 {
        match i {
            5 => Material::Light,
            3 | 4 => Material::Steel,
            _ => Material::Brass,
        }
    } else {
        match (i - 12) % 10 {
            2 => Material::Steel,
            3 | 7 | 8 => Material::Porcelain,
            9 => Material::Light,
            _ => Material::Brass,
        }
    }
}

fn staging(i: usize) -> V3 {
    let angle = i as f32 / PARTS as f32 * TAU * 5.0;
    let radius = 2.65 + random(i + 45) * 0.55;
    V3(
        angle.cos() * radius,
        0.18 + random(i + 1) * 0.13,
        angle.sin() * radius,
    )
}

fn pose(center: V3, axis: V3, size: V3) -> Pose {
    Pose {
        center,
        axis: axis.unit(),
        size,
    }
}
fn beam(a: V3, b: V3, width: f32, depth: f32) -> Pose {
    pose((a + b) * 0.5, b - a, V3(width, (b - a).length(), depth))
}

pub fn targets(form: usize, t: f32) -> Vec<Pose> {
    let mut parts = Vec::with_capacity(PARTS);
    let body = match form {
        0 => V3((t * 0.37).sin() * 0.22, 1.48 + (t * 5.2).cos() * 0.055, 0.0),
        1 => V3(0.0, 2.02 + (t * 2.4).sin() * 0.15, 0.0),
        _ => V3(0.0, 1.65, 0.0),
    };
    let axle = if form == 2 { V3::Y } else { V3::Z };
    parts.push(pose(body + axle * 0.17, axle, V3(0.46, 0.11, 0.46)));
    parts.push(pose(body - axle * 0.16, axle, V3(0.32, 0.13, 0.32)));
    parts.push(beam(
        body - V3(0.65, 0.0, 0.0),
        body + V3(0.65, 0.0, 0.0),
        0.13,
        0.13,
    ));
    parts.push(if form == 2 {
        beam(V3(0.0, 0.13, 0.0), body, 0.18, 0.18)
    } else {
        beam(
            body - V3(0.0, 0.0, 0.85),
            body + V3(0.0, 0.0, 0.85),
            0.23,
            0.23,
        )
    });
    parts.push(pose(
        body + V3(0.0, 0.12, -0.18),
        V3::Z,
        V3(0.42, 0.82, 0.26),
    ));
    parts.push(pose(
        body + V3(0.0, 0.05, 0.32),
        V3::Y,
        V3(0.15, 0.26, 0.12),
    ));
    for i in 0..6 {
        let a = i as f32 * TAU / 6.0;
        let offset = if form == 2 {
            V3(a.cos() * 0.63, 0.0, a.sin() * 0.63)
        } else {
            V3(a.cos() * 0.6, a.sin() * 0.45, 0.06)
        };
        parts.push(pose(body + offset, axle, V3(0.15, 0.08, 0.15)));
    }
    for module in 0..12 {
        let side = if module < 6 { -1.0 } else { 1.0 };
        let rib = (module % 6) as f32;
        let (root, elbow, tip) = match form {
            0 => {
                let row = (module % 6 / 2) as f32;
                let z = (row - 1.0) * 0.91 + ((module % 2) as f32 - 0.5) * 0.16;
                let phase = t * 2.6 + row * 2.1 + if side < 0.0 { PI } else { 0.0 };
                (
                    body + V3(side * 0.46, 0.0, z),
                    V3(side * (1.19 + phase.sin() * 0.08), 0.94, z + 0.1),
                    V3(
                        side * 1.88,
                        0.17 + phase.sin().max(0.0) * 0.53,
                        z + phase.cos() * 0.36,
                    ),
                )
            }
            1 => {
                let flap = (t * 2.4).sin();
                let z = (rib - 2.5) * 0.29;
                (
                    body + V3(side * 0.25, 0.0, z * 0.35),
                    body + V3(side * (1.16 + rib * 0.06), 0.18 + flap * 0.43, z),
                    body + V3(
                        side * (2.55 - rib * 0.11),
                        0.24 + flap * 0.98,
                        z * 1.85 - 0.1,
                    ),
                )
            }
            _ => {
                let a = module as f32 * TAU / 12.0 + t * 0.12;
                let open = (t * 0.75).sin() * 0.5 + 0.5;
                (
                    body + V3(a.cos() * 0.36, 0.0, a.sin() * 0.36),
                    body + V3(
                        a.cos() * (0.7 + open * 0.65),
                        0.68 - open * 0.18,
                        a.sin() * (0.7 + open * 0.65),
                    ),
                    body + V3(
                        a.cos() * (0.72 + open * 1.6),
                        1.45 - open * 1.1,
                        a.sin() * (0.72 + open * 1.6),
                    ),
                )
            }
        };
        let middle = root.lerp(elbow, 0.5);
        parts.push(beam(root, elbow, 0.10, 0.10));
        parts.push(beam(elbow, tip, 0.075, 0.075));
        parts.push(beam(
            root + V3(0.0, -0.12, 0.0),
            elbow.lerp(tip, 0.25),
            0.057,
            0.057,
        ));
        parts.push(beam(middle, tip.lerp(elbow, 0.55), 0.027, 0.027));
        let axle = if form == 2 {
            V3(-root.2, 0.0, root.0).unit()
        } else {
            V3::Z
        };
        parts.push(pose(root, axle, V3(0.19, 0.08, 0.19)));
        parts.push(pose(elbow, axle, V3(0.16, 0.07, 0.16)));
        parts.push(pose(tip, axle, V3(0.10, 0.065, 0.10)));
        let panel_width = if form == 0 { 0.15 } else { 0.27 };
        parts.push(beam(
            root.lerp(elbow, 0.25),
            root.lerp(elbow, 0.78),
            panel_width,
            0.07,
        ));
        parts.push(beam(
            elbow.lerp(tip, 0.2),
            elbow.lerp(tip, 0.82),
            panel_width * 1.25,
            0.055,
        ));
        parts.push(pose(tip + V3(0.0, 0.06, 0.0), V3::Y, V3(0.10, 0.09, 0.10)));
    }
    parts
}
