//! The puzzle lives in 3D. Connections are judged in the camera's 2D projection.
use std::{
    f64::consts::{PI, TAU},
    ops::{Add, Mul, Sub},
};

pub const CHAPTERS: usize = 3;
pub const GATES: usize = 3;
pub const TOLERANCE: f64 = 0.32;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl V3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    pub fn lerp(self, other: Self, t: f64) -> Self {
        self * (1.0 - t) + other * t
    }
}
impl Add for V3 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}
impl Sub for V3 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}
impl Mul<f64> for V3 {
    type Output = Self;
    fn mul(self, k: f64) -> Self {
        Self::new(self.x * k, self.y * k, self.z * k)
    }
}

/// Orthographic camera: zoom changes presentation, never the puzzle tolerance.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub yaw: f64,
    pub pitch: f64,
    pub zoom: f64,
    pub center: V3,
}
impl Camera {
    pub fn view(self, point: V3) -> V3 {
        let p = point - self.center;
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let (x, z) = (p.x * cy - p.z * sy, p.x * sy + p.z * cy);
        V3::new(x, p.y * cp - z * sp, p.y * sp + z * cp)
    }
    pub fn scale(self, w: f64, h: f64) -> f64 {
        (w * 0.075).min(h * 0.13) * self.zoom
    }
    pub fn project(self, point: V3, w: f64, h: f64) -> (f64, f64) {
        let p = self.view(point);
        let scale = self.scale(w, h);
        (w * 0.5 + p.x * scale, h * 0.49 - p.y * scale)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Align,
    Transit,
    Complete,
    Won,
}

#[derive(Clone)]
pub struct World {
    pub chapter: usize,
    pub gate: usize,
    pub phase: Phase,
    pub stations: [V3; 4],
    pub camera: Camera,
    pub target_yaw: f64,
    pub target_pitch: f64,
    pub target_zoom: f64,
    pub lock: f64,
    pub transit: f64,
    pub time: f64,
    pub reveal: f64,
    pub notice: f64,
    pub assists: usize,
    pub moves: usize,
    pub demo: bool,
    pub paused: bool,
    pub help: bool,
    wait: f64,
}

pub fn angle_delta(from: f64, to: f64) -> f64 {
    (to - from + PI).rem_euclid(TAU) - PI
}

impl Default for World {
    fn default() -> Self {
        let mut world = Self {
            chapter: 0,
            gate: 0,
            phase: Phase::Align,
            stations: [V3::default(); 4],
            camera: Camera {
                yaw: 0.0,
                pitch: 0.0,
                zoom: 1.0,
                center: V3::default(),
            },
            target_yaw: 0.0,
            target_pitch: 0.0,
            target_zoom: 1.0,
            lock: 0.0,
            transit: 0.0,
            time: 0.0,
            reveal: 0.0,
            notice: 0.0,
            assists: 0,
            moves: 0,
            demo: false,
            paused: false,
            help: false,
            wait: 0.0,
        };
        world.load(0);
        world
    }
}

impl World {
    pub fn title(&self) -> &'static str {
        [
            "THE TIDAL ARCHIVE",
            "GARDEN OF LOST SIGNALS",
            "THE LAST LIGHTHOUSE",
        ][self.chapter]
    }
    pub fn exit(&self, index: usize) -> V3 {
        self.stations[index] + V3::new(1.65, 0.25, 0.0)
    }
    pub fn entry(&self, index: usize) -> V3 {
        self.stations[index] + V3::new(-1.65, 0.25, 0.0)
    }
    pub fn pair(&self) -> (V3, V3) {
        (
            self.exit(self.gate.min(2)),
            self.entry(self.gate.min(2) + 1),
        )
    }
    pub fn solution(&self) -> (f64, f64) {
        let (a, b) = self.pair();
        let gap = b - a;
        (gap.x.atan2(gap.z), gap.y.atan2(gap.x.hypot(gap.z)))
    }
    pub fn error(&self) -> f64 {
        let (a, b) = self.pair();
        let gap = self.camera.view(b) - self.camera.view(a);
        V3::new(gap.x, gap.y, 0.0).length()
    }
    pub fn alignment(&self) -> f64 {
        (1.0 - self.error() / 4.0).clamp(0.0, 1.0)
    }
    pub fn ready(&self) -> bool {
        self.phase == Phase::Align && self.lock >= 1.0 && self.error() <= TOLERANCE
    }
    pub fn focus(&self) -> V3 {
        let (a, b) = self.pair();
        (a + b) * 0.5 + V3::new(0.0, -0.4, 0.0)
    }
    fn load(&mut self, chapter: usize) {
        self.chapter = chapter;
        self.gate = 0;
        self.phase = Phase::Align;
        self.lock = 0.0;
        self.wait = 0.0;
        self.reveal = 0.0;
        self.stations[0] = V3::new(-3.0, 0.0, 0.0);
        let orientations = [
            [(0.85_f64, 0.24_f64), (-0.68, -0.38), (1.95, 0.48)],
            [(-1.05, 0.50), (0.43, -0.46), (2.42, 0.32)],
            [(1.65, -0.48), (-1.7, 0.40), (0.15, 0.66)],
        ];
        for (i, (yaw, pitch)) in orientations[chapter].iter().copied().enumerate() {
            let distance = 4.2 + i as f64 * 0.45;
            let gap = V3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                yaw.cos() * pitch.cos(),
            ) * distance;
            self.stations[i + 1] = self.exit(i) + gap + V3::new(1.65, -0.25, 0.0);
        }
        let (yaw, pitch) = self.solution();
        self.target_yaw = yaw - 0.65;
        self.target_pitch = pitch + 0.32;
        self.camera.yaw = self.target_yaw;
        self.camera.pitch = self.target_pitch;
        self.camera.center = self.focus();
    }
    pub fn rotate(&mut self, yaw: f64, pitch: f64) {
        if self.paused || self.help || self.phase != Phase::Align {
            return;
        }
        self.demo = false;
        self.target_yaw = (self.target_yaw + yaw + PI).rem_euclid(TAU) - PI;
        self.target_pitch = (self.target_pitch + pitch).clamp(-1.1, 1.1);
        self.moves += 1;
    }
    pub fn zoom(&mut self, delta: f64) {
        if !self.help && !self.paused {
            self.target_zoom = (self.target_zoom + delta).clamp(0.65, 1.7);
        }
    }
    pub fn assist(&mut self) {
        if self.paused || self.help || self.phase != Phase::Align {
            return;
        }
        self.demo = false;
        (self.target_yaw, self.target_pitch) = self.solution();
        self.assists += 1;
    }
    pub fn action(&mut self) {
        if self.help {
            self.help = false;
            return;
        }
        if self.paused {
            self.paused = false;
            return;
        }
        match self.phase {
            Phase::Align if self.ready() => {
                // A pending orbit command must not drift the locked view mid-crossing.
                self.target_yaw = self.camera.yaw;
                self.target_pitch = self.camera.pitch;
                self.phase = Phase::Transit;
                self.transit = 0.0;
            }
            Phase::Align => self.notice = 2.0,
            Phase::Transit => {}
            Phase::Complete => self.load(self.chapter + 1),
            Phase::Won => self.restart(),
        }
    }
    pub fn restart(&mut self) {
        *self = Self {
            demo: self.demo,
            ..Self::default()
        };
    }
    pub fn traveler(&self) -> V3 {
        let from = self.stations[self.gate.min(3)] + V3::new(0.0, 0.65, 0.0);
        if self.phase != Phase::Transit {
            return from;
        }
        let to = self.stations[self.gate + 1] + V3::new(0.0, 0.65, 0.0);
        let (a, b) = self.pair();
        let t = self.transit;
        if t < 0.22 {
            from.lerp(a, smooth(t / 0.22))
        } else if t < 0.78 {
            let t = smooth((t - 0.22) / 0.56);
            let (sy, cy) = self.camera.yaw.sin_cos();
            let bow = V3::new(cy, 0.5, -sy) * ((t * PI).sin() * 1.5);
            a.lerp(b, t) + bow
        } else {
            b.lerp(to, smooth((t - 0.78) / 0.22))
        }
    }
    pub fn advance(&mut self, delta: f64) {
        if self.paused || self.help {
            return;
        }
        let dt = delta.clamp(0.0, 0.1);
        self.time += dt;
        self.wait += dt;
        self.notice = (self.notice - dt).max(0.0);
        self.reveal = (self.reveal + dt / 1.2).min(1.0);
        if self.demo {
            match self.phase {
                Phase::Align => {
                    let (yaw, pitch) = self.solution();
                    let orbit = (1.0 - self.wait / 2.8).max(0.0);
                    self.target_yaw = yaw - orbit * 0.7;
                    self.target_pitch = pitch + orbit * 0.35;
                    if self.ready() {
                        self.action();
                    }
                }
                Phase::Complete | Phase::Won if self.wait > 3.0 => self.action(),
                _ => {}
            }
        }
        let ease = 1.0 - (-dt * 9.0).exp();
        self.camera.yaw += angle_delta(self.camera.yaw, self.target_yaw) * ease;
        self.camera.pitch += (self.target_pitch - self.camera.pitch) * ease;
        self.camera.zoom += (self.target_zoom - self.camera.zoom) * ease;
        self.camera.center = self.camera.center.lerp(self.focus(), ease * 0.45);
        if self.phase == Phase::Align {
            self.lock = if self.error() <= TOLERANCE {
                (self.lock + dt / 0.4).min(1.0)
            } else {
                0.0
            };
        } else if self.phase == Phase::Transit {
            self.transit = (self.transit + dt / 2.6).min(1.0);
            if self.transit >= 1.0 {
                self.gate += 1;
                self.lock = 0.0;
                self.wait = 0.0;
                self.phase = if self.gate == GATES {
                    if self.chapter + 1 == CHAPTERS {
                        Phase::Won
                    } else {
                        Phase::Complete
                    }
                } else {
                    Phase::Align
                };
            }
        }
    }
}

fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fold_freezes_pending_camera_motion() {
        let mut w = World::default();
        w.assist();
        for _ in 0..120 {
            w.advance(1.0 / 60.0);
        }
        assert!(w.ready());
        w.rotate(0.4, 0.2);
        let (yaw, pitch) = (w.camera.yaw, w.camera.pitch);
        w.action();
        w.advance(0.1);
        assert_eq!(w.phase, Phase::Transit);
        assert_eq!((w.camera.yaw, w.camera.pitch), (yaw, pitch));
    }
    #[test]
    fn every_puzzle_has_a_real_depth_gap_and_a_projected_solution() {
        let mut w = World::default();
        for chapter in 0..CHAPTERS {
            w.load(chapter);
            for gate in 0..GATES {
                w.gate = gate;
                let (a, b) = w.pair();
                assert!((b - a).length() > 4.0);
                (w.camera.yaw, w.camera.pitch) = w.solution();
                assert!(w.error() < 1e-10);
                w.camera.yaw += 0.2;
                assert!(w.error() > TOLERANCE);
            }
        }
    }
    #[test]
    fn cannot_cross_until_aligned_and_held() {
        let mut w = World::default();
        w.action();
        assert_eq!(w.phase, Phase::Align);
        w.demo = true;
        w.assist();
        assert!(!w.demo);
        for _ in 0..80 {
            w.advance(1.0 / 60.0);
        }
        assert!(w.ready());
        w.action();
        assert_eq!(w.phase, Phase::Transit);
        let orientation = w.target_yaw;
        w.rotate(1.0, 1.0);
        assert_eq!(w.target_yaw, orientation);
        for _ in 0..170 {
            w.advance(1.0 / 60.0);
        }
        assert_eq!(w.gate, 1);
        assert_eq!(w.phase, Phase::Align);
    }
    #[test]
    fn zoom_cannot_cheat_and_pausing_freezes_the_crossing() {
        let mut w = World::default();
        let error = w.error();
        w.zoom(-0.35);
        for _ in 0..60 {
            w.advance(1.0 / 60.0);
        }
        assert!((w.error() - error).abs() < 1e-10);
        w.assist();
        for _ in 0..120 {
            w.advance(1.0 / 60.0);
        }
        w.action();
        w.paused = true;
        w.advance(0.1);
        assert_eq!(w.transit, 0.0);
        w.paused = false;
        w.help = true;
        w.advance(0.1);
        assert_eq!(w.transit, 0.0);
    }
    #[test]
    fn demo_completes_all_nine_folds_without_assistance() {
        let mut w = World {
            demo: true,
            ..World::default()
        };
        for _ in 0..60 * 90 {
            w.advance(1.0 / 60.0);
            if w.phase == Phase::Won {
                break;
            }
        }
        assert_eq!(w.phase, Phase::Won);
        assert_eq!(w.chapter, 2);
        assert_eq!(w.gate, 3);
        assert_eq!(w.assists, 0);
    }
    #[test]
    fn yaw_uses_shortest_arc() {
        assert!(angle_delta(PI - 0.01, -PI + 0.01).abs() < 0.03);
    }
}
