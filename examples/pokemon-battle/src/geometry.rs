use crate::{
    math::V3,
    raster::{Raster, Rgb},
};
use std::f32::consts::{PI, TAU};

/// A local coordinate frame: +Z is the face, +Y is up.
pub struct Model<'a> {
    pub raster: &'a mut Raster,
    pub origin: V3,
    pub yaw: f32,
}

impl Model<'_> {
    pub fn point(&self, p: V3) -> V3 {
        let (s, c) = self.yaw.sin_cos();
        self.origin + V3(p.0 * c + p.2 * s, p.1, p.2 * c - p.0 * s)
    }

    pub fn tri(&mut self, p: [V3; 3], color: Rgb, glow: bool) {
        self.raster.face(p.map(|v| self.point(v)), color, glow);
    }

    pub fn quad(&mut self, p: [V3; 4], color: Rgb, glow: bool) {
        self.tri([p[0], p[1], p[2]], color, glow);
        self.tri([p[0], p[2], p[3]], color, glow);
    }

    pub fn line(&mut self, a: V3, b: V3, color: Rgb) {
        self.raster.line(self.point(a), self.point(b), color);
    }

    pub fn ball(&mut self, p: V3, size: V3, color: Rgb) {
        let vertex = |i: usize, j: usize| {
            let u = i as f32 * TAU / 20.0;
            let v = j as f32 * PI / 12.0;
            p + V3(
                size.0 * v.sin() * u.cos(),
                size.1 * v.cos(),
                size.2 * v.sin() * u.sin(),
            )
        };
        for j in 0..12 {
            for i in 0..20 {
                self.quad(
                    [
                        vertex(i, j),
                        vertex(i + 1, j),
                        vertex(i + 1, j + 1),
                        vertex(i, j + 1),
                    ],
                    color,
                    false,
                );
            }
        }
    }

    pub fn tube(&mut self, a: V3, b: V3, ra: f32, rb: f32, color: Rgb, glow: bool) {
        let axis = (b - a).unit();
        let reference = if axis.1.abs() < 0.9 { V3::Y } else { V3::Z };
        let x = axis.cross(reference).unit();
        let y = axis.cross(x).unit();
        for i in 0..12 {
            let point = |p: V3, radius: f32, n: usize| {
                let t = n as f32 * TAU / 12.0;
                p + (x * t.cos() + y * t.sin()) * radius
            };
            let (u, v, w, z) = (
                point(a, ra, i),
                point(a, ra, i + 1),
                point(b, rb, i + 1),
                point(b, rb, i),
            );
            self.quad([u, v, w, z], color, glow);
            self.tri([a, v, u], color, glow);
            self.tri([b, z, w], color, glow);
        }
    }

    /// A hollow metal barrel, including the lip, inner wall and recessed bore.
    pub fn pipe(&mut self, a: V3, b: V3, radius: f32, color: Rgb) {
        let axis = (b - a).unit();
        let x = axis.cross(V3::Y).unit();
        let y = axis.cross(x).unit();
        let point = |p: V3, r: f32, i: usize| {
            let angle = i as f32 * TAU / 20.0;
            p + (x * angle.cos() + y * angle.sin()) * r
        };
        for i in 0..20 {
            self.quad(
                [
                    point(a, radius, i),
                    point(a, radius, i + 1),
                    point(b, radius, i + 1),
                    point(b, radius, i),
                ],
                color,
                false,
            );
            self.quad(
                [
                    point(b, radius, i),
                    point(b, radius, i + 1),
                    point(b, radius * 0.68, i + 1),
                    point(b, radius * 0.68, i),
                ],
                [0.79, 0.87, 0.89],
                false,
            );
            let back = a + axis * 0.08;
            self.quad(
                [
                    point(back, radius * 0.68, i),
                    point(back, radius * 0.68, i + 1),
                    point(b, radius * 0.68, i + 1),
                    point(b, radius * 0.68, i),
                ],
                [0.10, 0.17, 0.22],
                false,
            );
            self.tri(
                [
                    back,
                    point(back, radius * 0.68, i + 1),
                    point(back, radius * 0.68, i),
                ],
                [0.025, 0.05, 0.075],
                true,
            );
        }
    }

    pub fn ring(&mut self, p: V3, radius: f32, thickness: f32, color: Rgb) {
        for i in 0..96 {
            let vertex = |j: usize, r: f32| {
                let t = j as f32 * TAU / 96.0;
                p + V3(t.cos() * r, 0.0, t.sin() * r)
            };
            self.quad(
                [
                    vertex(i, radius),
                    vertex(i + 1, radius),
                    vertex(i + 1, radius - thickness),
                    vertex(i, radius - thickness),
                ],
                color,
                true,
            );
        }
    }

    pub fn disc(&mut self, p: V3, rx: f32, rz: f32, color: Rgb) {
        for i in 0..96 {
            let v = |j: usize| {
                let a = j as f32 * TAU / 96.0;
                p + V3(a.cos() * rx, 0.0, a.sin() * rz)
            };
            self.tri([p, v(i), v(i + 1)], color, true);
        }
    }
}
