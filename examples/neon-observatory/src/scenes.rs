//! Procedural scenes: light, depth and motion without image assets.
use std::f64::consts::{PI, TAU};

use rsille::{canvas::Canvas, tui::prelude::*};

use crate::{
    raster::{Light, Raster},
    BG,
};

pub(crate) fn draw_scene(
    surface: &mut Canvas,
    dimensions: CanvasContext,
    scene: usize,
    time: f64,
    a: Color,
    b: Color,
) {
    if dimensions.dot_width() < 2 || dimensions.dot_height() < 2 {
        return;
    }
    let mut image = Raster::new(
        dimensions.dot_width() as usize,
        dimensions.dot_height() as usize,
        BG,
    );
    let (a, b) = (Light::emission(a), Light::emission(b));
    match scene {
        1 => warp(&mut image, time, a, b),
        2 => terrain(&mut image, time, a, b),
        3 => galaxy(&mut image, time, a, b),
        4 => torus(&mut image, time, a, b),
        5 => aurora(&mut image, time, a, b),
        _ => orbit(&mut image, time, a, b),
    }
    image.bloom();
    image.resolve(surface);
}

fn hash(seed: usize) -> f64 {
    ((seed as f64 * 127.1 + 311.7).sin() * 43758.5453)
        .fract()
        .abs()
}

fn stars(image: &mut Raster, time: f64, count: usize, strength: f64) {
    for i in 0..count {
        let x = hash(i * 7 + 1) * image.width as f64;
        let y = hash(i * 7 + 2) * image.height as f64;
        let twinkle = 0.7 + 0.3 * (time * (0.5 + hash(i + 41)) + i as f64).sin();
        let light = Light(0.48, 0.64, 0.9).mix(Light::WHITE, hash(i + 91));
        image.glow(
            x,
            y,
            0.5 + hash(i + 17) * 0.35,
            light * (strength * twinkle),
        );
        if i % 47 == 0 {
            image.streak((x - 2.2, y), (x + 2.2, y), light * strength * 0.25);
            image.streak((x, y - 2.2), (x, y + 2.2), light * strength * 0.25);
        }
    }
}

fn orbit(image: &mut Raster, time: f64, a: Light, b: Light) {
    let (w, h) = (image.width as f64, image.height as f64);
    let radius = (w * 0.22).min(h * 0.335);
    let (cx, cy) = (w * 0.51, h * 0.49);
    stars(image, time, 135, 0.65);
    let tilt = -0.23 + (time * 0.09).sin() * 0.08;
    let (st, ct) = tilt.sin_cos();
    for py in 0..image.height {
        for px in 0..image.width {
            let (x, y) = ((px as f64 - cx) / radius, (py as f64 - cy) / radius);
            let distance = x.hypot(y);
            // Orthographic rays intersect a tilted ring plane and an opaque sphere.
            let (rx, ry) = (x * ct + y * st, -x * st + y * ct);
            let ring_z = -ry * 1.95;
            let ring_r = (rx * rx + ry * ry + ring_z * ring_z).sqrt();
            let sphere_z = if distance < 1.0 {
                (1.0 - distance * distance).sqrt()
            } else {
                -10.0
            };
            if distance < 1.0 {
                let longitude = x.atan2(sphere_z) + time * 0.24;
                let storm = (longitude * 7.0 + y * 13.0).sin() * (y * 9.0 - longitude * 2.0).cos();
                let bands =
                    (y * 27.0 + storm * 1.2 + (longitude * 3.0).sin() * 1.1).sin() * 0.5 + 0.5;
                let diffuse = (-x * 0.58 - y * 0.47 + sphere_z * 0.66).max(0.0);
                let surface = b.mix(a, bands * 0.8).mix(Light::WHITE, bands.powi(5) * 0.4);
                let rim = (1.0 - sphere_z).powi(4);
                let ring_shadow = if (1.26..2.05)
                    .contains(&((x + 0.24).powi(2) + (y * 2.5 + 0.3).powi(2)).sqrt())
                {
                    0.65
                } else {
                    1.0
                };
                image.set(
                    px,
                    py,
                    surface * ((0.06 + diffuse * 1.65) * ring_shadow) + a * rim * 0.65,
                );
            } else {
                let atmosphere = (-((distance - 1.0) / 0.07).powi(2)).exp();
                image.add(px, py, a * atmosphere * 0.55);
            }
            if (1.28..2.12).contains(&ring_r) && ring_z > sphere_z {
                let grooves = 0.5 + 0.5 * (ring_r * 160.0).sin();
                let gap = if (1.69..1.77).contains(&ring_r) || (1.94..1.98).contains(&ring_r) {
                    0.12
                } else {
                    1.0
                };
                let angular = 0.8 + 0.2 * (rx.atan2(ring_z) * 9.0 - time * 0.6).sin();
                let density = (0.38 + grooves * 0.75) * gap * angular;
                let hue = a.mix(b, ((ring_r - 1.28) / 0.84).clamp(0.0, 1.0));
                image.set(px, py, hue.mix(Light::WHITE, 0.26) * density);
            }
        }
    }
    // A moon crosses the ring's outer edge on a much slower orbit.
    let angle = time * 0.2;
    let (mx, my) = (
        cx + angle.cos() * radius * 2.45,
        cy + angle.sin() * radius * 0.65,
    );
    image.glow(mx, my, 1.4, Light::WHITE * 1.5);
}

fn warp(image: &mut Raster, time: f64, a: Light, b: Light) {
    let (w, h) = (image.width as f64, image.height as f64);
    let center = (
        w * (0.5 + (time * 0.27).sin() * 0.12),
        h * (0.5 + (time * 0.31).cos() * 0.1),
    );
    let scale = h * 0.48;
    // Twisting tunnel ribs, drawn from the distant throat toward the camera.
    for ring in (0..24).rev() {
        let z = (ring as f64 / 24.0 - time * 0.32).rem_euclid(1.0);
        let radius = scale * 0.21 / (z + 0.045);
        let twist = time * 0.25 + z * 2.7;
        let hue = a.mix(b, (z * 4.0 + time * 0.5).sin() * 0.5 + 0.5);
        let light = hue * (0.32 + (1.0 - z).powi(2) * 0.8);
        let point = |angle: f64| {
            let radius = radius * (1.0 + 0.08 * (angle * 6.0 + twist).sin());
            (
                center.0 + angle.cos() * radius * 1.5,
                center.1 + angle.sin() * radius,
            )
        };
        for step in 0..100 {
            let angle = step as f64 / 100.0 * TAU + twist;
            if step % 12 < 9 {
                image.streak(point(angle), point(angle + TAU / 100.0), light);
            }
        }
    }
    for i in 0..330 {
        let angle = hash(i + 500) * TAU + time * 0.12;
        let depth = (hash(i + 1000) - time * (0.29 + hash(i + 9) * 0.12)).rem_euclid(1.0);
        let radius = scale * (0.16 + hash(i + 400) * 0.5) / (depth + 0.025);
        let tail = radius * (0.72 + depth * 0.25);
        let hue = a.mix(b, hash(i + 1500)).mix(Light::WHITE, 0.45);
        image.streak(
            (
                center.0 + tail * angle.cos() * 1.5,
                center.1 + tail * angle.sin(),
            ),
            (
                center.0 + radius * angle.cos() * 1.5,
                center.1 + radius * angle.sin(),
            ),
            hue * (0.4 + (1.0 - depth).powi(2) * 3.0),
        );
    }
    image.glow(center.0, center.1, h * 0.08, a * 0.45);
    image.glow(center.0, center.1, 1.4, Light::WHITE * 3.2);
}

fn galaxy(image: &mut Raster, time: f64, a: Light, b: Light) {
    let (w, h) = (image.width as f64, image.height as f64);
    let radius = (w * 0.43).min(h * 0.96);
    let (cx, cy) = (w * 0.5, h * 0.5);
    let tilt: f64 = -0.27;
    let (st, ct) = tilt.sin_cos();
    stars(image, time, 155, 0.55);
    for y in 0..image.height {
        for x in 0..image.width {
            let (dx, dy) = ((x as f64 - cx) / radius, (y as f64 - cy) / radius);
            let (gx, gy) = (dx * ct + dy * st, (-dx * st + dy * ct) / 0.46);
            let r = gx.hypot(gy);
            let angle = gy.atan2(gx);
            let phase = angle * 3.0 - (r + 0.055).ln() * 6.0 + time * 0.2;
            let arm = ((phase.cos() - 1.0) * 3.2).exp();
            let dust = (((phase + 0.48).cos() - 1.0) * 12.0).exp();
            let grain = 0.7 + 0.3 * (gx * 173.0 + (gy * 119.0).sin() * 2.0).sin();
            let edge = (1.0 - (r / 1.1).powi(8)).clamp(0.0, 1.0);
            let disk = (-r * 1.8).exp() * (0.1 + arm * 2.8) * grain * (1.0 - dust * 0.83) * edge;
            let core = (-r * r * 90.0).exp();
            let halo = (-r * r * 11.0).exp();
            let hue = a.mix(b, (r * 1.2).min(1.0));
            image.add(
                x,
                y,
                hue * disk + Light(1.0, 0.69, 0.36) * (core * 3.7 + halo * 0.24),
            );
        }
    }
    for i in 0..2100 {
        let r = hash(i + 2100).sqrt() * 0.98;
        let angle = (i % 3) as f64 * TAU / 3.0 + (r + 0.055).ln() * 2.0 - time / 15.0
            + (hash(i + 4000) - 0.5) * (0.15 + r * 0.27);
        let (gx, gy) = (r * angle.cos(), r * angle.sin() * 0.46);
        let (x, y) = (
            cx + radius * (gx * ct - gy * st),
            cy + radius * (gx * st + gy * ct),
        );
        let light = a.mix(b, r).mix(Light::WHITE, hash(i + 6200) * 0.8);
        let twinkle = 0.8 + 0.2 * (time * 1.3 + i as f64).sin();
        image.glow(
            x,
            y,
            if i % 71 == 0 { 0.9 } else { 0.42 },
            light * (0.2 + hash(i + 8000) * 0.8) * twinkle,
        );
    }
    image.glow(cx, cy, 1.2, Light::WHITE * 4.0);
}

fn rotate(p: [f64; 3], spin: f64, tilt: f64) -> [f64; 3] {
    let (ss, cs) = spin.sin_cos();
    let (st, ct) = tilt.sin_cos();
    let (x, y) = (p[0] * cs - p[1] * ss, p[0] * ss + p[1] * cs);
    [x, y * ct - p[2] * st, y * st + p[2] * ct]
}

fn torus(image: &mut Raster, time: f64, a: Light, b: Light) {
    let (w, h) = (image.width as f64, image.height as f64);
    let scale = (w * 0.25).min(h * 0.31);
    let (spin, tilt) = (time * 0.28, 0.65 + (time * 0.32).sin() * 0.75);
    let (ss, cs) = spin.sin_cos();
    let (st, ct) = tilt.sin_cos();
    let local = |p: [f64; 3]| {
        let (y, z) = (p[1] * ct + p[2] * st, -p[1] * st + p[2] * ct);
        [p[0] * cs + y * ss, -p[0] * ss + y * cs, z]
    };
    stars(image, time, 100, 0.4);
    for y in 0..image.height {
        for x in 0..image.width {
            let (sx, sy) = ((x as f64 - w * 0.5) / scale, (y as f64 - h * 0.5) / scale);
            if sx * sx + sy * sy > 2.35 {
                continue;
            }
            let mut distance = 0.0;
            let mut hit = None;
            for _ in 0..52 {
                let p = local([sx, sy, 3.2 - distance]);
                let radial = p[0].hypot(p[1]);
                let sdf = (radial - 1.02).hypot(p[2]) - 0.39;
                if sdf < 0.007 {
                    hit = Some((p, radial));
                    break;
                }
                distance += sdf.max(0.003);
                if distance > 6.0 {
                    break;
                }
            }
            if let Some((p, radial)) = hit {
                let normal = rotate(
                    [
                        p[0] / radial * (radial - 1.02) / 0.39,
                        p[1] / radial * (radial - 1.02) / 0.39,
                        p[2] / 0.39,
                    ],
                    spin,
                    tilt,
                );
                let diffuse = (-normal[0] * 0.45 - normal[1] * 0.6 + normal[2] * 0.65).max(0.0);
                let specular = (-normal[0] * 0.25 - normal[1] * 0.32 + normal[2] * 0.91)
                    .max(0.0)
                    .powi(36);
                let u = p[1].atan2(p[0]);
                let v = p[2].atan2(radial - 1.02);
                let flow = (u * 6.0 + v * 3.0 - time * 1.9).sin() * 0.5 + 0.5;
                let stripe = (v * 5.0 + u * 3.0 + time).sin().abs();
                let grid = (u * 18.0).sin().abs().min((v * 9.0).sin().abs());
                let fresnel = (1.0 - normal[2].max(0.0)).powi(3);
                let hue = a.mix(b, flow);
                let chrome = hue * (0.16 + diffuse * 1.2)
                    + Light::WHITE * (specular * 4.0 + stripe.powi(12) * 0.2);
                let engraving = if grid < 0.12 { 0.48 } else { 1.0 };
                let seam = if stripe < 0.1 { 1.6 } else { 0.0 };
                image.set(x, y, chrome * engraving + hue * (fresnel * 1.4 + seam));
            }
        }
    }
    // Orbiting sparks trace the reactor's magnetic field without obscuring its surface.
    for particle in 0..3 {
        for trail in (0..50).rev() {
            let angle = time * 0.8 + particle as f64 * TAU / 3.0 - trail as f64 * 0.018;
            let p = rotate([angle.cos() * 1.83, angle.sin() * 1.83, 0.0], -0.3, 0.38);
            let fade = (1.0 - trail as f64 / 50.0).powi(2);
            image.glow(
                w * 0.5 + p[0] * scale,
                h * 0.5 + p[1] * scale,
                0.65,
                a.mix(b, particle as f64 / 2.0) * fade * 1.7,
            );
        }
    }
}

fn aurora(image: &mut Raster, time: f64, a: Light, b: Light) {
    let (w, h) = (image.width as f64, image.height as f64);
    let horizon = h * 0.73;
    stars(image, time, 155, 0.7);
    for y in horizon as usize..image.height {
        for x in 0..image.width {
            image.set(x, y, Light(0.014, 0.025, 0.047));
        }
    }
    for x in 0..image.width {
        let u = x as f64 / w;
        for layer in (0..3).rev() {
            let depth = layer as f64;
            let fold = (u * 11.0 + time * 0.3 + depth * 1.8).sin();
            let curtain = 0.47
                + depth * 0.045
                + (u * 7.0 + time * 0.22 + depth).sin() * 0.13
                + (u * 17.0 - time * 0.31).sin() * 0.045;
            let fringe = curtain * h;
            let length = h * (0.2 + 0.1 * (fold * 0.5 + 0.5));
            let pleat = 0.35 + 0.65 * (u * 135.0 + fold * 7.0 + time * 0.45).sin().abs().powi(3);
            let hue = a.mix(b, (fold * 0.5 + 0.5) * 0.35 + depth * 0.1);
            for y in 0..horizon as usize {
                let above = fringe - y as f64;
                if above > -2.0 && above < length {
                    let edge = (-above.abs() / 2.0).exp();
                    let veil = (-above.max(0.0) / (length * 0.38)).exp()
                        * (above / length * PI).sin().max(0.0);
                    let light = hue * ((veil * 2.1 + edge * 1.8) * pleat / (1.0 + depth * 0.5));
                    let violet = b * (above / length).max(0.0).powi(2) * veil * 1.2;
                    image.add(x, y, light + violet);
                    // Fragment the mirrored curtains into moving horizontal lake ripples.
                    let reflected = (horizon + (horizon - y as f64) * 0.53) as usize;
                    if reflected < image.height {
                        let ripple =
                            (reflected as f64 * 2.7 + u * 23.0 + time * 1.7).sin() * 0.5 + 0.5;
                        image.add(x, reflected, light * (0.16 + ripple.powi(5) * 0.35));
                    }
                }
            }
        }
    }
    // Two opaque mountain ridges give the curtains scale and a clear silhouette.
    for x in 0..image.width {
        let u = x as f64 / w;
        let far = horizon
            - h * (0.035 + (u * 18.0 + 0.4).sin().abs() * 0.06 + (u * 39.0).sin().abs() * 0.02);
        let near = horizon - h * (0.01 + (u * 13.0 + 2.0).sin().abs() * 0.045);
        for y in far as usize..horizon as usize + 1 {
            image.set(
                x,
                y,
                if (y as f64) < near {
                    Light(0.025, 0.055, 0.09)
                } else {
                    Light(0.012, 0.025, 0.045)
                },
            );
        }
        for y in horizon as usize + 1..image.height {
            let ripple = (u * 70.0 + y as f64 * 2.3 + time).sin().max(0.0).powi(14);
            image.add(x, y, a * ripple * 0.025);
        }
    }
}

#[derive(Clone, Copy)]
struct TerrainVertex {
    screen: [f64; 2],
    inverse_z: f64,
    x_over_z: f64,
    shade: f64,
}

struct TerrainShader {
    a: Light,
    b: Light,
    time: f64,
    focal: f64,
}

impl TerrainShader {
    fn light(&self, x: f64, z: f64, shade: f64) -> Light {
        let world_z = z + self.time * 1.25;
        let distance_to_line = |v: f64, period: f64| {
            let phase = v.rem_euclid(period);
            phase.min(period - phase)
        };
        // Perspective-correct world coordinates keep the grid attached to the hills.
        let line_distance = distance_to_line(x, 0.65).min(distance_to_line(world_z, 0.8));
        let line_width = (z / self.focal).clamp(0.02, 0.24);
        let grid = (-(line_distance / line_width).powi(2) * 1.4).exp();
        let pulse = ((z * 0.65 + self.time * 1.8).rem_euclid(6.0) - 3.0) / 0.32;
        let scan = (-pulse * pulse).exp();
        let path = (-(x / 0.2).powi(2)).exp();
        let fog = (-z * 0.055).exp();
        let hue = self.a.mix(self.b, (z / 28.0).min(1.0));
        let ground = self.b * (0.035 + shade.powi(3) * 0.33) + self.a * shade.powi(6) * 0.08;
        let wires = hue * grid * (1.1 + shade * 1.2 + scan * 3.6);
        let signal = self.a * (scan * 0.85 + path * (0.6 + scan * 2.0));
        let highlight = Light::WHITE * grid * scan * 0.5;
        (ground + wires + signal + highlight) * fog + self.b * ((1.0 - fog) * 0.11)
    }

    fn triangle(&self, image: &mut Raster, depth: &mut [f64], vertices: [TerrainVertex; 3]) {
        let [a, b, c] = vertices;
        let edge = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
            (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
        };
        let area = edge(a.screen, b.screen, c.screen);
        if area.abs() < 1e-8 {
            return;
        }
        let x0 = a.screen[0]
            .min(b.screen[0])
            .min(c.screen[0])
            .floor()
            .max(0.0) as usize;
        let x1 = (a.screen[0]
            .max(b.screen[0])
            .max(c.screen[0])
            .ceil()
            .max(0.0) as usize)
            .min(image.width);
        let y0 = a.screen[1]
            .min(b.screen[1])
            .min(c.screen[1])
            .floor()
            .max(0.0) as usize;
        let y1 = (a.screen[1]
            .max(b.screen[1])
            .max(c.screen[1])
            .ceil()
            .max(0.0) as usize)
            .min(image.height);
        for y in y0..y1 {
            for x in x0..x1 {
                let sample = [x as f64 + 0.5, y as f64 + 0.5];
                let wa = edge(b.screen, c.screen, sample) / area;
                let wb = edge(c.screen, a.screen, sample) / area;
                let wc = 1.0 - wa - wb;
                if wa < -1e-7 || wb < -1e-7 || wc < -1e-7 {
                    continue;
                }
                let inverse_z = wa * a.inverse_z + wb * b.inverse_z + wc * c.inverse_z;
                let z = 1.0 / inverse_z;
                let index = y * image.width + x;
                if z >= depth[index] {
                    continue;
                }
                depth[index] = z;
                let world_x = (wa * a.x_over_z + wb * b.x_over_z + wc * c.x_over_z) * z;
                let shade = wa * a.shade + wb * b.shade + wc * c.shade;
                image.set(x, y, self.light(world_x, z, shade));
            }
        }
    }
}

fn terrain(image: &mut Raster, time: f64, a: Light, b: Light) {
    let (w, h) = (image.width as f64, image.height as f64);
    let horizon = h * 0.44;
    let focal = h * 0.95;
    let sun = (w * 0.5, h * 0.235, h * 0.155);
    stars(image, time, 155, 0.55);
    for y in 0..image.height {
        for x in 0..image.width {
            let dx = (x as f64 - sun.0) / sun.2;
            let dy = (y as f64 - sun.1) / sun.2;
            let halo = (-(dx * dx + dy * dy) * 0.7).exp();
            let haze = (-((y as f64 - horizon) / (h * 0.13)).powi(2)).exp();
            image.add(x, y, b * halo * 0.18 + a.mix(b, 0.7) * haze * 0.1);
            if dx * dx + dy * dy < 1.0 {
                let band = (dy + 1.0) * 0.5;
                if band < 0.48 || (band * 11.0).fract() > 0.15 + band * 0.3 {
                    let warm = Light(1.6, 0.42, 0.12);
                    image.set(x, y, b.mix(warm, 0.75 - band * 0.6) * 1.45);
                }
            }
        }
    }
    // Distant silhouettes sit behind the depth-tested, continuously moving valley.
    for x in 0..image.width {
        let u = x as f64 / w;
        let ridge = horizon
            - h * (0.035 + (u * 19.0).sin().abs() * 0.035 + (u * 43.0 + 0.8).sin().abs() * 0.012);
        for y in ridge as usize..horizon as usize + 1 {
            image.set(x, y, b * 0.12 + a * 0.02);
        }
    }

    let height_at = |x: f64, z: f64| {
        let z = z + time * 1.25;
        let banks = 1.0 - (-x * x * 0.13).exp();
        banks
            * (1.05
                + (x * 0.83 + z * 0.29).sin() * (z * 0.56).cos() * 0.55
                + (x * 2.3 + z * 0.7).sin() * 0.16)
            + (z * 1.9 + x * 0.5).sin() * 0.08
    };
    let shader = TerrainShader { a, b, time, focal };
    let columns = (image.width / 2).clamp(12, 180);
    let rows = 144;
    let mut vertices = Vec::with_capacity((columns + 1) * (rows + 1));
    let mut depth = vec![f64::INFINITY; image.width * image.height];
    for row in 0..=rows {
        let z = 0.85 * (48.0_f64 / 0.85).powf(row as f64 / rows as f64);
        for column in 0..=columns {
            let screen_x = column as f64 / columns as f64 * w;
            let x = (screen_x - w * 0.5) * z / focal;
            let height = height_at(x, z);
            let slope_x = (height_at(x + 0.04, z) - height) / 0.04;
            let slope_z = (height_at(x, z + 0.04) - height) / 0.04;
            let normal_length = (1.0 + slope_x * slope_x + slope_z * slope_z).sqrt();
            let shade = ((slope_x * 0.4 + 0.8 - slope_z * 0.45) / normal_length).max(0.0);
            vertices.push(TerrainVertex {
                screen: [screen_x, horizon + (1.5 - height) * focal / z],
                inverse_z: 1.0 / z,
                x_over_z: x / z,
                shade,
            });
        }
    }
    for row in 0..rows {
        for column in 0..columns {
            let i = row * (columns + 1) + column;
            let j = i + columns + 1;
            shader.triangle(
                image,
                &mut depth,
                [vertices[i], vertices[j], vertices[i + 1]],
            );
            shader.triangle(
                image,
                &mut depth,
                [vertices[i + 1], vertices[j], vertices[j + 1]],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{INLINE_HEIGHT, INLINE_WIDTH};
    use rsille::render::{area::Area, buffer::Buffer, chunk::Chunk, Draw};

    #[test]
    fn terrain_hides_distant_surfaces_in_either_drawing_order() {
        let shader = TerrainShader {
            a: Light(0.1, 1.2, 1.4),
            b: Light(0.8, 0.2, 1.3),
            time: 12.0,
            focal: 16.0,
        };
        let triangle = |z: f64| {
            [[1.0, 1.0], [11.0, 1.0], [1.0, 7.0]].map(|screen| TerrainVertex {
                screen,
                inverse_z: 1.0 / z,
                x_over_z: (screen[0] - 6.0) / shader.focal,
                shade: 0.8,
            })
        };
        let render = |distances: &[f64]| {
            let mut image = Raster::new(12, 8, BG);
            let mut depth = vec![f64::INFINITY; 12 * 8];
            for &z in distances {
                shader.triangle(&mut image, &mut depth, triangle(z));
            }
            assert_eq!(depth[2 * 12 + 2], 1.0);
            let mut canvas = Canvas::new();
            image.resolve(&mut canvas);
            canvas
        };
        let foreground = render(&[1.0]);
        assert_eq!(foreground, render(&[2.0, 1.0]));
        assert_eq!(foreground, render(&[1.0, 2.0]));
    }

    fn render(scene: usize, time: f64, palette: usize, width: u16, height: u16) -> Buffer {
        let mut canvas = Canvas::new();
        canvas.set_bound((0, width as i32 - 1), (0, height as i32 - 1));
        canvas.fixed_bound(true);
        let (a, b) = crate::State {
            palette,
            ..crate::State::default()
        }
        .colors();
        draw_scene(
            &mut canvas,
            CanvasContext::new(width, height),
            scene,
            time,
            a,
            b,
        );
        let size = (width, height).into();
        let mut buffer = Buffer::new(size);
        canvas
            .draw(Chunk::new(&mut buffer, Area::new((0, 0).into(), size)).unwrap())
            .unwrap();
        buffer
    }

    #[test]
    fn every_scene_is_repeatable_animated_and_responds_to_palette() {
        for scene in 0..6 {
            let original = render(scene, 12.0, 0, INLINE_WIDTH - 2, INLINE_HEIGHT - 9);
            let repeated = render(scene, 12.0, 0, INLINE_WIDTH - 2, INLINE_HEIGHT - 9);
            assert_eq!(
                original.content(),
                repeated.content(),
                "scene {scene} must not flicker at a frozen time"
            );
            for changed in [
                render(scene, 14.0, 0, INLINE_WIDTH - 2, INLINE_HEIGHT - 9),
                render(scene, 12.0, 1, INLINE_WIDTH - 2, INLINE_HEIGHT - 9),
                render(scene, 12.0, 2, INLINE_WIDTH - 2, INLINE_HEIGHT - 9),
            ] {
                let differences = original
                    .content()
                    .iter()
                    .zip(changed.content())
                    .filter(|(a, b)| a != b)
                    .count();
                assert!(
                    differences > 30,
                    "scene {scene} must visibly respond to time and each palette"
                );
            }
        }
    }

    #[test]
    fn every_scene_renders_at_small_sizes_and_different_orientations() {
        for scene in 0..6 {
            for (width, height) in [(1, 1), (3, 2), (12, 4), (40, 12)] {
                for time in [0.0, 0.7, 45.0] {
                    let buffer = render(scene, time, 0, width, height);
                    assert!(
                        buffer.content().iter().any(|cell| cell
                            .content
                            .as_char()
                            .is_some_and(|c| ('\u{2801}'..='\u{28ff}').contains(&c))),
                        "scene {scene}, {width}x{height}, time {time}"
                    );
                }
            }
        }
    }
}
