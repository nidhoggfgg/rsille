//! Lit floating architecture, projected from real 3D geometry into braille dots.
use crate::{
    raster::{Light, Raster},
    world::{Camera, Phase, World, V3},
};
use rsille::{canvas::Canvas, tui::prelude::*};
use std::f64::consts::{PI, TAU};

pub const BG: Color = Color::Rgb(5, 10, 22);
pub const TEXT: Color = Color::Rgb(218, 234, 243);
pub const MUTED: Color = Color::Rgb(108, 139, 163);

pub fn colors(chapter: usize) -> (Color, Color) {
    [
        (Color::Rgb(72, 231, 218), Color::Rgb(255, 181, 111)),
        (Color::Rgb(177, 146, 255), Color::Rgb(244, 216, 128)),
        (Color::Rgb(104, 189, 255), Color::Rgb(255, 131, 178)),
    ][chapter]
}

fn hash(i: usize) -> f64 {
    ((i as f64 * 127.1 + 311.7).sin() * 43758.5453)
        .fract()
        .abs()
}

struct Face {
    points: [V3; 4],
    normal: V3,
    light: Light,
}

fn box_faces(faces: &mut Vec<Face>, center: V3, size: V3, light: Light) {
    let h = size * 0.5;
    let p = |x, y, z| center + V3::new(h.x * x, h.y * y, h.z * z);
    for (points, normal) in [
        (
            [
                p(-1.0, 1.0, -1.0),
                p(1.0, 1.0, -1.0),
                p(1.0, 1.0, 1.0),
                p(-1.0, 1.0, 1.0),
            ],
            V3::new(0.0, 1.0, 0.0),
        ),
        (
            [
                p(-1.0, -1.0, -1.0),
                p(1.0, -1.0, -1.0),
                p(1.0, -1.0, 1.0),
                p(-1.0, -1.0, 1.0),
            ],
            V3::new(0.0, -1.0, 0.0),
        ),
        (
            [
                p(1.0, -1.0, -1.0),
                p(1.0, 1.0, -1.0),
                p(1.0, 1.0, 1.0),
                p(1.0, -1.0, 1.0),
            ],
            V3::new(1.0, 0.0, 0.0),
        ),
        (
            [
                p(-1.0, -1.0, -1.0),
                p(-1.0, 1.0, -1.0),
                p(-1.0, 1.0, 1.0),
                p(-1.0, -1.0, 1.0),
            ],
            V3::new(-1.0, 0.0, 0.0),
        ),
        (
            [
                p(-1.0, -1.0, 1.0),
                p(1.0, -1.0, 1.0),
                p(1.0, 1.0, 1.0),
                p(-1.0, 1.0, 1.0),
            ],
            V3::new(0.0, 0.0, 1.0),
        ),
        (
            [
                p(-1.0, -1.0, -1.0),
                p(1.0, -1.0, -1.0),
                p(1.0, 1.0, -1.0),
                p(-1.0, 1.0, -1.0),
            ],
            V3::new(0.0, 0.0, -1.0),
        ),
    ] {
        faces.push(Face {
            points,
            normal,
            light,
        });
    }
}

fn triangle(image: &mut Raster, p: [(f64, f64); 3], light: Light) {
    let cross = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    };
    let area = cross(p[0], p[1], p[2]);
    if area.abs() < 0.01 {
        return;
    }
    let x0 = p
        .iter()
        .map(|p| p.0)
        .fold(f64::INFINITY, f64::min)
        .floor()
        .max(0.0) as usize;
    let x1 = (p
        .iter()
        .map(|p| p.0)
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        .max(0.0) as usize)
        .min(image.width);
    let y0 = p
        .iter()
        .map(|p| p.1)
        .fold(f64::INFINITY, f64::min)
        .floor()
        .max(0.0) as usize;
    let y1 = (p
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        .max(0.0) as usize)
        .min(image.height);
    for y in y0..y1 {
        for x in x0..x1 {
            let q = (x as f64 + 0.5, y as f64 + 0.5);
            let a = cross(p[1], p[2], q) / area;
            let b = cross(p[2], p[0], q) / area;
            let c = 1.0 - a - b;
            if a >= -0.001 && b >= -0.001 && c >= -0.001 {
                let grain = 0.97 + 0.03 * (x as f64 * 0.4 + y as f64 * 0.7).sin();
                image.set(x, y, light * grain);
            }
        }
    }
}

struct Painter<'a> {
    image: &'a mut Raster,
    camera: Camera,
}
impl Painter<'_> {
    fn screen(&self, p: V3) -> (f64, f64) {
        self.camera
            .project(p, self.image.width as f64, self.image.height as f64)
    }
    fn line(&mut self, a: V3, b: V3, light: Light) {
        self.image.streak(self.screen(a), self.screen(b), light);
    }
    fn glow(&mut self, p: V3, radius: f64, light: Light) {
        let (x, y) = self.screen(p);
        self.image.glow(x, y, radius, light);
    }
    fn ring(&mut self, center: V3, radius: f64, tilt: f64, phase: f64, light: Light) {
        let point = |angle: f64| {
            center
                + V3::new(
                    angle.cos() * radius,
                    angle.sin() * radius * tilt.sin(),
                    angle.sin() * radius * tilt.cos(),
                )
        };
        for i in 0..64 {
            let angle = i as f64 / 64.0 * TAU + phase;
            if i % 16 < 13 {
                self.line(point(angle), point(angle + TAU / 64.0), light);
            }
        }
    }
    fn portal(&mut self, p: V3, radius: f64, phase: f64, light: Light) {
        let center = self.screen(p);
        let scale = self
            .camera
            .scale(self.image.width as f64, self.image.height as f64);
        let radius = radius * scale;
        for i in 0..64 {
            if i % 16 > 12 {
                continue;
            }
            let angle = i as f64 / 64.0 * TAU + phase;
            let next = angle + TAU / 64.0;
            self.image.streak(
                (
                    center.0 + angle.cos() * radius,
                    center.1 + angle.sin() * radius,
                ),
                (
                    center.0 + next.cos() * radius,
                    center.1 + next.sin() * radius,
                ),
                light,
            );
        }
        self.image.glow(center.0, center.1, 1.0, light * 1.8);
    }
}

pub fn draw(canvas: &mut Canvas, dimensions: CanvasContext, world: &World) {
    let (w, h) = (
        dimensions.dot_width() as usize,
        dimensions.dot_height() as usize,
    );
    if w < 2 || h < 4 {
        return;
    }
    let mut image = Raster::new(w, h, BG);
    let (primary, secondary) = colors(world.chapter);
    let (a, b) = (Light::emission(primary), Light::emission(secondary));
    let t = world.time;
    // A luminous nebula, not a flat terminal background.
    for y in 0..h {
        for x in 0..w {
            let nx = (x as f64 / w as f64 - 0.5) * 2.0;
            let ny = (y as f64 / h as f64 - 0.5) * 2.0;
            let cloud = (-nx * nx * 2.0 - ny * ny * 3.0).exp();
            let veil = (ny * 5.0 + nx * 3.0 + t * 0.12).sin() * 0.5 + 0.5;
            image.add(x, y, a.mix(b, veil) * (cloud * 0.018));
        }
    }
    for i in 0..180 {
        let x = (hash(i * 7) * w as f64 + world.camera.yaw * 2.0).rem_euclid(w as f64);
        let y = (hash(i * 7 + 1) * h as f64 + world.camera.pitch * 2.0).rem_euclid(h as f64);
        let strength = 0.2 + hash(i * 7 + 2) * 0.6;
        image.glow(
            x,
            y,
            0.35 + hash(i * 7 + 3) * 0.3,
            Light::WHITE * (strength * (0.75 + 0.25 * (t + i as f64).sin())),
        );
    }
    let camera = world.camera;
    let mut painter = Painter {
        image: &mut image,
        camera,
    };
    let focus = world.focus();
    // Orbital scaffolding makes rotations legible even when portals overlap.
    for ring in [4.9, 5.2, 6.4] {
        painter.ring(
            focus + V3::new(0.0, -2.7, 0.0),
            ring,
            0.0,
            t * 0.025,
            a * 0.13,
        );
    }
    for i in 0..24 {
        let angle = i as f64 / 24.0 * TAU;
        let p = focus + V3::new(angle.cos() * 5.2, -2.7, angle.sin() * 5.2);
        painter.line(p, p + V3::new(0.0, 0.15, 0.0), a * 0.4);
    }
    let mut faces = Vec::new();
    for (index, position) in world.stations.iter().copied().enumerate() {
        let active = index == world.gate || index == world.gate + 1;
        let hue = if index <= world.gate { a } else { b };
        let strength = if active { 1.0 } else { 0.25 };
        let stone = Light(0.3, 0.43, 0.55).mix(hue, 0.22) * strength;
        box_faces(&mut faces, position, V3::new(3.0, 0.38, 2.3), stone * 1.8);
        box_faces(
            &mut faces,
            position + V3::new(0.0, -0.48, 0.0),
            V3::new(2.35, 0.58, 1.7),
            stone,
        );
        box_faces(
            &mut faces,
            position + V3::new(0.0, -0.98, 0.0),
            V3::new(1.45, 0.45, 1.1),
            stone * 0.7,
        );
        // Staircase and an impossible lintel turn the platforms into tiny buildings.
        for step in 0..5 {
            box_faces(
                &mut faces,
                position + V3::new(-0.8 + step as f64 * 0.3, 0.3 + step as f64 * 0.12, -0.65),
                V3::new(0.35, 0.2 + step as f64 * 0.22, 0.7),
                stone * 1.7,
            );
        }
        for x in [-1.05, 1.05] {
            box_faces(
                &mut faces,
                position + V3::new(x, 1.0, 0.7),
                V3::new(0.18, 1.8, 0.18),
                stone * 1.7,
            );
        }
        box_faces(
            &mut faces,
            position + V3::new(0.0, 1.85, 0.7),
            V3::new(2.35, 0.18, 0.32),
            stone * 2.0,
        );
    }
    faces.sort_by(|a, b| {
        let depth = |face: &Face| face.points.iter().map(|p| camera.view(*p).z).sum::<f64>();
        depth(a).total_cmp(&depth(b))
    });
    for face in faces {
        let normal = camera.view(face.normal + camera.center);
        if normal.z <= 0.0 {
            continue;
        }
        let lighting =
            (face.normal.x * -0.35 + face.normal.y * 0.75 + face.normal.z * 0.4).max(0.0);
        let color = face.light * (0.34 + lighting * 1.15);
        let points = face.points.map(|p| painter.screen(p));
        triangle(painter.image, [points[0], points[1], points[2]], color);
        triangle(painter.image, [points[0], points[2], points[3]], color);
        for i in 0..4 {
            painter
                .image
                .streak(points[i], points[(i + 1) % 4], color * 0.22);
        }
    }
    // Filigree is emissive and intentionally readable through the architecture.
    for (index, position) in world.stations.iter().copied().enumerate() {
        let hue = if index <= world.gate { a } else { b };
        let active = index == world.gate || index == world.gate + 1;
        let light = hue * if active { 1.1 } else { 0.18 };
        for z in [-1.15, 1.15] {
            painter.line(
                position + V3::new(-1.5, 0.22, z),
                position + V3::new(1.5, 0.22, z),
                light * 0.7,
            );
        }
        for x in [-1.5, 1.5] {
            painter.line(
                position + V3::new(x, 0.22, -1.15),
                position + V3::new(x, 0.22, 1.15),
                light * 0.7,
            );
        }
        painter.ring(
            position + V3::new(0.0, -1.5, 0.0),
            1.35,
            0.2,
            t * 0.25 + index as f64,
            light * 0.6,
        );
        painter.ring(
            position + V3::new(0.0, 2.15, 0.7),
            0.55,
            1.2,
            t * -0.5,
            light * 0.65,
        );
        for strand in 0..7 {
            let angle = strand as f64 / 7.0 * TAU + t * 0.15;
            let start = position + V3::new(angle.cos() * 0.7, -1.2, angle.sin() * 0.7);
            let end = start
                + V3::new(
                    (t + angle).sin() * 0.2,
                    -1.0 - hash(strand + index * 7),
                    0.0,
                );
            painter.line(start, end, light * 0.2);
            painter.glow(end, 0.7, light * 0.8);
        }
        if world.chapter == 1 {
            // The signal garden grows helical antennae from the roof.
            for i in 0..50 {
                let s = i as f64 / 50.0;
                let angle = s * TAU * 2.0 + t * 0.3;
                painter.glow(
                    position + V3::new(angle.cos() * 0.55, 2.0 + s * 1.8, angle.sin() * 0.55),
                    0.65,
                    light,
                );
            }
        } else if world.chapter == 2 {
            painter.line(
                position + V3::new(0.0, 1.9, 0.7),
                position + V3::new(0.0, 4.2, 0.7),
                light * 0.3,
            );
            painter.glow(position + V3::new(0.0, 3.0, 0.7), 2.5, light * 1.8);
        }
    }
    let (from, to) = world.pair();
    let close = world.error() < 0.65;
    let portal_light = if world.ready() || world.phase == Phase::Transit {
        Light::WHITE * 2.5
    } else {
        a
    };
    if world.phase == Phase::Align || world.phase == Phase::Transit {
        // A dotted parallax vector disappears as the two depths collapse.
        if !close {
            for i in 0..24 {
                if i % 3 != 0 {
                    continue;
                }
                let s = (i as f64 / 24.0 + t * 0.06).fract();
                painter.glow(from.lerp(to, s), 0.6, a.mix(b, s) * 0.45);
            }
        }
        painter.portal(from, 0.5, t * 0.45, portal_light);
        painter.portal(to, 0.67, -t * 0.45, if close { portal_light } else { b });
        if close {
            painter.portal(
                (from + to) * 0.5,
                0.95 + world.lock * 0.15,
                t * 0.2,
                a * world.lock * 0.7,
            );
        }
    }
    let traveler = world.traveler();
    painter.glow(traveler, 2.4, Light::WHITE * 3.5);
    painter.portal(traveler, 0.18, t, Light::WHITE);
    if world.phase == Phase::Transit {
        let progress = world.transit;
        let pulse = (progress * PI).sin();
        for i in 0..140 {
            let phase = hash(i) * TAU + t * 0.3;
            let r = (hash(i + 201) * 4.0 + 0.3) * (0.5 + progress * 1.5);
            let p = traveler + V3::new(phase.cos() * r, phase.sin() * r, (hash(i + 400) - 0.5) * r);
            painter.glow(p, 0.6, a.mix(b, hash(i + 500)) * (pulse * 1.8));
        }
        for i in 0..8 {
            let r = ((progress * 3.0 + i as f64 / 8.0).fract() * 3.5 + 0.4) * pulse;
            painter.portal(
                (from + to) * 0.5,
                r,
                i as f64 + t,
                a.mix(b, i as f64 / 8.0) * (pulse * 0.5),
            );
        }
        painter.line(from, to, Light::WHITE * pulse * 1.5);
    }
    // Orientation compass is geometry too, and stays attached to the lower left.
    let origin = (11.0, h as f64 - 12.0);
    for (axis, light) in [
        (V3::new(1.0, 0.0, 0.0), Light(1.0, 0.25, 0.4)),
        (V3::new(0.0, 1.0, 0.0), a),
        (V3::new(0.0, 0.0, 1.0), Light(0.3, 0.5, 1.0)),
    ] {
        let axis = camera.view(axis + camera.center);
        painter.image.streak(
            origin,
            (origin.0 + axis.x * 8.0, origin.1 - axis.y * 8.0),
            light,
        );
    }
    image.bloom();
    image.resolve(canvas);
}
