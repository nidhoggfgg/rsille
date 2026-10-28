//! Small, depth-tested CPU renderer for solid mechanical parts.
use crate::{
    machine::{Machine, Material, Part},
    math::V3,
};
use std::f32::consts::TAU;

pub type Rgb = [f32; 3];

#[derive(Clone, Copy)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.62,
            pitch: 0.38,
            distance: 9.8,
        }
    }
}

#[derive(Default)]
pub struct Raster {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Rgb>,
    depth: Vec<f32>,
    eye: V3,
    right: V3,
    up: V3,
    forward: V3,
    focal: f32,
}

#[derive(Clone, Copy)]
struct Screen {
    x: f32,
    y: f32,
    z: f32,
}

impl Raster {
    pub fn render(&mut self, width: usize, height: usize, machine: &Machine, camera: Camera) {
        self.width = width;
        self.height = height;
        self.pixels.resize(width * height, [0.0; 3]);
        self.depth.resize(width * height, f32::INFINITY);
        self.depth.fill(f32::INFINITY);
        let target = V3(0.0, 1.1, 0.0);
        self.eye = target
            + V3(
                camera.yaw.sin() * camera.pitch.cos(),
                camera.pitch.sin(),
                camera.yaw.cos() * camera.pitch.cos(),
            ) * camera.distance;
        self.forward = (target - self.eye).unit();
        self.right = self.forward.cross(V3::Y).unit();
        self.up = self.right.cross(self.forward).unit();
        self.focal = (height as f32 * 1.6).min(width as f32 * 1.05);
        for y in 0..height {
            for x in 0..width {
                let u = (x as f32 / width.max(1) as f32 - 0.5) * 2.0;
                let v = y as f32 / height.max(1) as f32;
                let halo = (-(u * u * 2.0 + (v - 0.55).powi(2) * 3.0)).exp();
                self.pixels[y * width + x] = [
                    0.018 + halo * 0.018,
                    0.023 + halo * 0.028,
                    0.035 + halo * 0.039,
                ];
            }
        }
        self.platform(machine.time as f32);
        // Contact shadows are drawn before the elevated geometry and share its depth buffer.
        for part in &machine.parts {
            let p = part.pose.center;
            let radius = (part.pose.size.1 * 0.15 + part.pose.size.0 * 0.3).clamp(0.08, 0.36);
            self.disc(
                V3(p.0 + p.1 * 0.24, -0.015, p.2 - p.1 * 0.2),
                V3::Y,
                radius,
                [0.027, 0.038, 0.047],
            );
        }
        for part in &machine.parts {
            self.part(part);
        }
    }

    fn project(&self, p: V3) -> Option<Screen> {
        let v = p - self.eye;
        let z = v.dot(self.forward);
        if z < 0.1 {
            return None;
        }
        Some(Screen {
            x: self.width as f32 * 0.5 + v.dot(self.right) * self.focal / z,
            y: self.height as f32 * 0.46 - v.dot(self.up) * self.focal / z,
            z,
        })
    }

    fn triangle(&mut self, points: [V3; 3], rgb: Rgb) {
        let [Some(a), Some(b), Some(c)] = points.map(|p| self.project(p)) else {
            return;
        };
        let edge = |a: Screen, b: Screen, x: f32, y: f32| {
            (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
        };
        let area = edge(a, b, c.x, c.y);
        if area.abs() < 1e-6 {
            return;
        }
        let minx = a.x.min(b.x).min(c.x).floor().max(0.0) as usize;
        let maxx = a.x.max(b.x).max(c.x).ceil().max(0.0).min(self.width as f32) as usize;
        let miny = a.y.min(b.y).min(c.y).floor().max(0.0) as usize;
        let maxy =
            a.y.max(b.y)
                .max(c.y)
                .ceil()
                .max(0.0)
                .min(self.height as f32) as usize;
        for y in miny..maxy {
            for x in minx..maxx {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let wa = edge(b, c, px, py) / area;
                let wb = edge(c, a, px, py) / area;
                let wc = 1.0 - wa - wb;
                if wa < -1e-5 || wb < -1e-5 || wc < -1e-5 {
                    continue;
                }
                let z = 1.0 / (wa / a.z + wb / b.z + wc / c.z);
                let i = y * self.width + x;
                if z < self.depth[i] {
                    self.depth[i] = z;
                    self.pixels[i] = rgb;
                }
            }
        }
    }

    fn quad(&mut self, p: [V3; 4], color: Rgb) {
        self.triangle([p[0], p[1], p[2]], color);
        self.triangle([p[0], p[2], p[3]], color);
    }

    fn line(&mut self, a: V3, b: V3, color: Rgb) {
        let (Some(a), Some(b)) = (self.project(a), self.project(b)) else {
            return;
        };
        let steps = (b.x - a.x).abs().max((b.y - a.y).abs()).ceil() as usize;
        // Bound work even if a future scene supplies an offscreen segment.
        if steps > 4 * (self.width + self.height) {
            return;
        }
        for i in 0..=steps {
            let t = i as f32 / steps.max(1) as f32;
            let x = (a.x + (b.x - a.x) * t).round() as i32;
            let y = (a.y + (b.y - a.y) * t).round() as i32;
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                continue;
            }
            let z = 1.0 / ((1.0 - t) / a.z + t / b.z);
            let k = y as usize * self.width + x as usize;
            if z < self.depth[k] + 0.012 {
                self.pixels[k] = color;
                self.depth[k] = z;
            }
        }
    }

    fn platform(&mut self, t: f32) {
        for i in 0..96 {
            let a = i as f32 * TAU / 96.0;
            let b = (i + 1) as f32 * TAU / 96.0;
            let point = |angle: f32, y| V3(angle.cos() * 3.45, y, angle.sin() * 3.45);
            self.triangle(
                [V3(0.0, -0.04, 0.0), point(a, -0.04), point(b, -0.04)],
                [0.072, 0.095, 0.113],
            );
            let normal = V3(a.cos(), 0.0, a.sin());
            let rgb = self.shade(Material::Steel, normal, point(a, -0.15));
            self.quad(
                [
                    point(a, -0.04),
                    point(a, -0.29),
                    point(b, -0.29),
                    point(b, -0.04),
                ],
                rgb,
            );
            for radius in [3.20, 3.31] {
                self.line(
                    V3(a.cos() * radius, -0.025, a.sin() * radius),
                    V3(b.cos() * radius, -0.025, b.sin() * radius),
                    [0.20, 0.19, 0.15],
                );
            }
            if i % 4 == 0 {
                self.line(
                    V3(a.cos() * 2.94, -0.025, a.sin() * 2.94),
                    V3(a.cos() * 3.1, -0.025, a.sin() * 3.1),
                    [0.29, 0.24, 0.15],
                );
            }
        }
        // A quiet clock hand makes the workbench feel powered even during assembly.
        let a = t * 0.08;
        self.line(
            V3(a.cos() * 2.98, -0.023, a.sin() * 2.98),
            V3(a.cos() * 3.18, -0.023, a.sin() * 3.18),
            [0.24, 0.59, 0.61],
        );
    }

    fn shade(&self, material: Material, normal: V3, p: V3) -> Rgb {
        let base = match material {
            Material::Brass => [0.78, 0.46, 0.20],
            Material::Porcelain => [0.79, 0.84, 0.79],
            Material::Steel => [0.17, 0.26, 0.32],
            Material::Light => return [0.25, 0.91, 0.94],
        };
        let light = V3(-0.45, 0.82, 0.55).unit();
        let diffuse = normal.dot(light).max(0.0);
        let view = (self.eye - p).unit();
        let half = (light + view).unit();
        let spec = normal.dot(half).max(0.0).powi(28) * 0.55;
        let rim = normal.dot(V3(0.6, 0.2, -0.8).unit()).max(0.0) * 0.17;
        std::array::from_fn(|i| (base[i] * (0.28 + diffuse * 0.82 + rim) + spec).min(1.0))
    }

    fn basis(axis: V3) -> (V3, V3) {
        let reference = if axis.dot(V3::Z).abs() < 0.9 {
            V3::Z
        } else {
            V3(1.0, 0.0, 0.0)
        };
        let x = axis.cross(reference).unit();
        (x, x.cross(axis).unit())
    }

    fn part(&mut self, part: &Part) {
        let pose = part.pose;
        let (x, z) = Self::basis(pose.axis);
        if part.gear {
            let (s, c) = part.spin.sin_cos();
            self.gear(part, x * c + z * s, z * c - x * s);
            return;
        }
        let axes = [
            x * pose.size.0 * 0.5,
            pose.axis * pose.size.1 * 0.5,
            z * pose.size.2 * 0.5,
        ];
        let vertices: [V3; 8] = std::array::from_fn(|i| {
            pose.center
                + axes[0] * if i & 1 == 0 { -1.0 } else { 1.0 }
                + axes[1] * if i & 2 == 0 { -1.0 } else { 1.0 }
                + axes[2] * if i & 4 == 0 { -1.0 } else { 1.0 }
        });
        for (face, normal) in [
            ([1, 3, 7, 5], x),
            ([0, 4, 6, 2], -x),
            ([2, 6, 7, 3], pose.axis),
            ([0, 1, 5, 4], -pose.axis),
            ([4, 5, 7, 6], z),
            ([0, 2, 3, 1], -z),
        ] {
            let center = pose.center + normal * 0.01;
            if normal.dot(self.eye - center) < 0.0 {
                continue;
            }
            let rgb = self.shade(part.material, normal, pose.center);
            self.quad(face.map(|i| vertices[i]), rgb);
        }
    }

    fn gear(&mut self, part: &Part, x: V3, z: V3) {
        let p = part.pose;
        for i in 0..40 {
            let a = i as f32 * TAU / 40.0;
            let b = (i + 1) as f32 * TAU / 40.0;
            let radius = |index: usize| {
                if matches!(index % 4, 1 | 2) {
                    p.size.0
                } else {
                    p.size.0 * 0.79
                }
            };
            let va = (x * a.cos() + z * a.sin()) * radius(i);
            let vb = (x * b.cos() + z * b.sin()) * radius(i + 1);
            for sign in [-1.0, 1.0] {
                let normal = p.axis * sign;
                let center = p.center + normal * p.size.1 * 0.5;
                if normal.dot(self.eye - center) > 0.0 {
                    self.triangle(
                        [center, center + va, center + vb],
                        self.shade(part.material, normal, center),
                    );
                }
            }
            let normal = (va + vb).unit();
            let rgb = self.shade(part.material, normal, p.center);
            self.quad(
                [
                    p.center + va - p.axis * p.size.1 * 0.5,
                    p.center + va + p.axis * p.size.1 * 0.5,
                    p.center + vb + p.axis * p.size.1 * 0.5,
                    p.center + vb - p.axis * p.size.1 * 0.5,
                ],
                rgb,
            );
        }
        for sign in [-1.0, 1.0] {
            let normal = p.axis * sign;
            let center = p.center + normal * (p.size.1 * 0.5 + 0.003);
            if normal.dot(self.eye - center) > 0.0 {
                self.disc(center, normal, p.size.0 * 0.32, [0.08, 0.13, 0.15]);
                self.disc(
                    center + normal * 0.005,
                    normal,
                    p.size.0 * 0.13,
                    [0.47, 0.70, 0.71],
                );
            }
        }
    }

    fn disc(&mut self, center: V3, normal: V3, radius: f32, color: Rgb) {
        let (x, z) = Self::basis(normal);
        for i in 0..16 {
            let a = i as f32 * TAU / 16.0;
            let b = (i + 1) as f32 * TAU / 16.0;
            self.triangle(
                [
                    center,
                    center + (x * a.cos() + z * a.sin()) * radius,
                    center + (x * b.cos() + z * b.sin()) * radius,
                ],
                color,
            );
        }
    }
}
