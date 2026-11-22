//! Analytic motion: rendering a timestamp never advances or randomizes the scene.
use std::f32::consts::{PI, TAU};

type Rgb = [f32; 3];

#[derive(Default)]
pub struct Sea {
    pub pixels: Vec<Rgb>,
    width: usize,
    height: usize,
    scale: f32,
}

fn seed(n: usize) -> f32 {
    let mut x = (n as u32).wrapping_mul(747796405).wrapping_add(2891336453);
    x = ((x >> ((x >> 28) + 4)) ^ x).wrapping_mul(277803737);
    ((x >> 22) ^ x) as f32 / u32::MAX as f32
}

impl Sea {
    pub fn render(&mut self, width: usize, height: usize, time: f64) {
        self.width = width;
        self.height = height;
        // Letterbox the composition in unusually narrow windows without stretching it.
        self.scale = (height as f32).min(width as f32 / 1.15).max(1.0);
        self.pixels.resize(width * height, [0.0; 3]);
        let t = time as f32;
        let aspect = width as f32 / self.scale;
        for y in 0..height {
            let v = y as f32 / height.max(1) as f32;
            for x in 0..width {
                let u = x as f32 / width.max(1) as f32;
                let beam = (u * 17.0 + v * 2.5 + (t * 0.12).sin() * 0.6)
                    .sin()
                    .max(0.0)
                    .powi(12)
                    + 0.5 * (u * 31.0 + v * 3.0 - t * 0.08).sin().max(0.0).powi(18);
                let light = beam * (1.0 - v).powi(2);
                let vignette = (1.0 - (u - 0.5).powi(2) * 1.8).max(0.2);
                self.pixels[y * width + x] = [
                    (0.008 + light * 0.012) * vignette,
                    (0.024 + (1.0 - v) * 0.052 + light * 0.075) * vignette,
                    (0.065 + (1.0 - v) * 0.075 + light * 0.10) * vignette,
                ];
            }
        }

        // Small distant fish travel as one gently undulating school.
        for i in 0..12 {
            let x = ((t * 0.018 + seed(i + 300) * 0.42).rem_euclid(aspect + 0.5)) - 0.25;
            let y = 0.50 + seed(i + 400) * 0.15 + (t * 0.45 + x * 5.0).sin() * 0.025;
            let color = [0.012, 0.060, 0.083];
            self.line((x - 0.013, y), (x + 0.009, y), 0.0018, color, 1.0);
            self.line((x - 0.013, y), (x - 0.020, y - 0.006), 0.0015, color, 1.0);
        }

        // Two larger subjects keep the bell details readable in a short inline strip.
        for (i, (x, y, radius, color)) in [
            (0.71, 0.39, 0.23, [0.96, 0.38, 0.51]),
            (0.29, 0.34, 0.25, [0.25, 0.75, 1.00]),
        ]
        .into_iter()
        .enumerate()
        {
            let phase = i as f32 * 2.39;
            let x = (x + (t * 0.42 + phase).sin() * 0.07) * aspect;
            let y = y + (t * 0.65 + phase).sin() * 0.10;
            self.jelly((x, y), radius, color, t, phase);
        }

        for i in 0..45 {
            let speed = 0.007 + seed(i + 90) * 0.012;
            let x = seed(i) * aspect + (t * 0.18 + i as f32).sin() * 0.012;
            let y = (seed(i + 180) - t * speed).rem_euclid(1.18) - 0.09;
            let alpha = 0.15 + 0.25 * (t * 0.7 + i as f32).sin().powi(2);
            self.glow(
                (x, y),
                0.002 + seed(i + 270) * 0.002,
                [0.20, 0.58, 0.70],
                alpha,
            );
        }
    }

    fn jelly(&mut self, center: (f32, f32), radius: f32, color: Rgb, t: f32, phase: f32) {
        let beat = t * 1.9 + phase;
        let pulse = (beat.sin() * 0.5 + 0.5).powi(3);
        let r = radius * (1.0 - 0.32 * pulse);
        let dome = r * (0.58 + 0.55 * pulse);
        let (cx, cy) = (center.0, center.1 - pulse * radius * 0.20);
        self.glow((cx, cy), r * 1.7, color, 0.065);

        // Thin trailing tentacles carry a delayed traveling wave down their lengths.
        for arm in 0..11 {
            let a = arm as f32 / 10.0;
            let root = (a * 2.0 - 1.0) * r * 0.84;
            let length = radius * (1.3 + seed(arm + (phase * 10.0) as usize) * 0.7);
            let mut last = (cx + root, cy);
            for step in 1..=48 {
                let s = step as f32 / 48.0;
                let curl = (s * 9.0 - t * 1.8 + phase + a * 3.0).sin();
                let x = cx
                    + root * (1.0 - s * 0.32)
                    + s * radius * (0.65 * curl - 0.65 * (t * 0.42 + phase).cos());
                let y = cy + s * length;
                let strength = (1.0 - s).powf(0.55) * (0.34 + 0.12 * (beat - s * 6.0).sin());
                self.line(last, (x, y), radius * 0.014, color, strength);
                last = (x, y);
            }
        }

        // Four thicker, ruffled oral arms make the silhouettes less like umbrellas.
        for arm in 0..4 {
            let a = arm as f32 - 1.5;
            let mut last = (cx + a * r * 0.20, cy);
            for step in 1..=42 {
                let s = step as f32 / 42.0;
                let x = cx
                    + a * r * 0.20
                    + radius
                        * s
                        * (0.48 * (s * 13.0 - t * 1.8 + a).sin() - 0.45 * (t * 0.42 + phase).cos());
                let y = cy + s * radius * (1.25 + 0.12 * a);
                let width =
                    radius * (0.045 + 0.023 * (s * 43.0 + a * 2.0 - t).sin()) * (1.0 - 0.7 * s);
                self.line(last, (x, y), width, color, 0.25 * (1.0 - s * 0.7));
                last = (x, y);
            }
        }

        // Translucent bell: elliptical dome with scalloped skirt and radial ribs.
        let (px, py) = self.project((cx, cy));
        let pr = r * self.scale;
        let pd = dome * self.scale;
        let min_x = (px - pr - 2.0).max(0.0) as usize;
        let max_x = ((px + pr + 2.0).max(0.0) as usize).min(self.width);
        let min_y = (py - pd - 2.0).max(0.0) as usize;
        let max_y = ((py + pr * 0.22 + 2.0).max(0.0) as usize).min(self.height);
        for y in min_y..max_y {
            for x in min_x..max_x {
                let u = (x as f32 + 0.5 - px) / pr;
                let v = (y as f32 + 0.5 - py) / pd;
                let top = -(1.0 - u * u).max(0.0).sqrt();
                let bottom =
                    0.13 * (1.0 - u * u).max(0.0).sqrt() + 0.025 * (u * 34.0 + phase).cos();
                if u.abs() > 1.0 || v < top || v > bottom {
                    continue;
                }
                let edge = (-((v - top) * pd / 1.8).powi(2)).exp();
                let rim = (-((v - bottom) * pd / 1.5).powi(2)).exp();
                let ribs = (u.atan2(-v * 0.8) * 9.0 + phase * 0.2).cos().powi(12);
                let inner = (-((u * 2.6).powi(2) + ((v + 0.25) * 3.5).powi(2))).exp();
                let light =
                    0.13 + edge * 0.48 + rim * 0.46 + ribs * 0.11 + inner * (0.30 + pulse * 0.15);
                let pixel = &mut self.pixels[y * self.width + x];
                for c in 0..3 {
                    pixel[c] = pixel[c] * 0.76 + color[c] * light;
                }
            }
        }

        // A dotted luminous rim ripples around the opening.
        for i in 0..65 {
            let angle = i as f32 / 64.0 * PI;
            let x = cx + angle.cos() * r;
            let y = cy + angle.sin() * r * 0.10;
            self.glow(
                (x, y),
                radius * 0.018,
                color,
                0.21 + 0.10 * (angle * 12.0 + beat).sin(),
            );
        }
        // Four visible horseshoe-shaped organs inside each transparent bell.
        for organ in 0..4 {
            let angle = organ as f32 * TAU / 4.0 + 0.4;
            let ox = cx + angle.cos() * r * 0.24;
            let oy = cy - dome * 0.24 + angle.sin() * r * 0.10;
            let mut last = (ox + r * 0.11, oy);
            for step in 1..=16 {
                let a = step as f32 / 16.0 * PI * 1.65;
                let next = (ox + a.cos() * r * 0.11, oy + a.sin() * r * 0.065);
                self.line(last, next, radius * 0.014, color, 0.32);
                last = next;
            }
        }
    }

    fn project(&self, p: (f32, f32)) -> (f32, f32) {
        (
            p.0 * self.scale,
            p.1 * self.scale + (self.height as f32 - self.scale) * 0.5,
        )
    }

    fn glow(&mut self, center: (f32, f32), radius: f32, color: Rgb, strength: f32) {
        self.line(center, center, radius, color, strength);
    }

    /// Soft capsule rasterization, bounded to its footprint rather than the screen.
    fn line(&mut self, a: (f32, f32), b: (f32, f32), radius: f32, color: Rgb, strength: f32) {
        let (ax, ay) = self.project(a);
        let (bx, by) = self.project(b);
        let r = (radius * self.scale).max(0.55);
        let pad = r * 2.5;
        let min_x = (ax.min(bx) - pad).max(0.0) as usize;
        let max_x = ((ax.max(bx) + pad + 1.0).max(0.0) as usize).min(self.width);
        let min_y = (ay.min(by) - pad).max(0.0) as usize;
        let max_y = ((ay.max(by) + pad + 1.0).max(0.0) as usize).min(self.height);
        let dx = bx - ax;
        let dy = by - ay;
        let len = (dx * dx + dy * dy).max(0.0001);
        for y in min_y..max_y {
            for x in min_x..max_x {
                let vx = x as f32 + 0.5 - ax;
                let vy = y as f32 + 0.5 - ay;
                let s = ((vx * dx + vy * dy) / len).clamp(0.0, 1.0);
                let dist = (vx - s * dx).powi(2) + (vy - s * dy).powi(2);
                let alpha = (-dist / (r * r)).exp() * strength;
                let pixel = &mut self.pixels[y * self.width + x];
                for c in 0..3 {
                    pixel[c] += color[c] * alpha;
                }
            }
        }
    }
}
