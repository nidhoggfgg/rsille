//! Tiny vector rasterizer. Coordinates describe a 128 × 64 stage, y down.
use std::f32::consts::TAU;

pub type Rgb = [f32; 3];
pub type Point = (f32, f32);
pub const INK: Rgb = [0.08, 0.07, 0.16];
pub const WHITE: Rgb = [0.96, 0.96, 1.0];

#[derive(Default)]
pub struct Raster {
    pub pixels: Vec<Rgb>,
    width: usize,
    height: usize,
    scale: f32,
    offset: Point,
}

impl Raster {
    pub fn begin(&mut self, width: usize, height: usize, time: f32) {
        self.width = width;
        self.height = height;
        self.scale = (width as f32 / 128.0).min(height as f32 / 64.0).max(0.001);
        self.offset = (
            (width as f32 - 128.0 * self.scale) * 0.5,
            (height as f32 - 64.0 * self.scale) * 0.5,
        );
        self.pixels.resize(width * height, [0.0; 3]);
        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width.max(1) as f32;
                let v = y as f32 / height.max(1) as f32;
                let beam = (u * 23.0 + v * 2.0 + time * TAU / 24.0)
                    .sin()
                    .max(0.0)
                    .powi(12)
                    * 0.035;
                self.pixels[y * width + x] =
                    [0.035 + beam, 0.035 + beam * 0.8, 0.09 + v * 0.045 + beam];
            }
        }
    }

    fn bounds(&self, min: Point, max: Point) -> (usize, usize, usize, usize) {
        let p = |value: f32, offset: f32, limit: usize| {
            ((value * self.scale + offset).max(0.0) as usize).min(limit)
        };
        (
            p(min.0, self.offset.0, self.width),
            p(min.1, self.offset.1, self.height),
            p(max.0 + 1.0, self.offset.0, self.width),
            p(max.1 + 1.0, self.offset.1, self.height),
        )
    }

    fn world(&self, x: usize, y: usize) -> Point {
        (
            (x as f32 + 0.5 - self.offset.0) / self.scale,
            (y as f32 + 0.5 - self.offset.1) / self.scale,
        )
    }

    fn blend(&mut self, x: usize, y: usize, color: Rgb, alpha: f32) {
        let pixel = &mut self.pixels[y * self.width + x];
        for c in 0..3 {
            pixel[c] = pixel[c] * (1.0 - alpha) + color[c] * alpha;
        }
    }

    pub fn ellipse(&mut self, center: Point, rx: f32, ry: f32, color: Rgb, alpha: f32) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let (x0, y0, x1, y1) = self.bounds(
            (center.0 - rx, center.1 - ry),
            (center.0 + rx, center.1 + ry),
        );
        for y in y0..y1 {
            for x in x0..x1 {
                let p = self.world(x, y);
                if ((p.0 - center.0) / rx).powi(2) + ((p.1 - center.1) / ry).powi(2) <= 1.0 {
                    self.blend(x, y, color, alpha);
                }
            }
        }
    }

    pub fn line(&mut self, a: Point, b: Point, width: f32, color: Rgb) {
        let (x0, y0, x1, y1) = self.bounds(
            (a.0.min(b.0) - width, a.1.min(b.1) - width),
            (a.0.max(b.0) + width, a.1.max(b.1) + width),
        );
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = (dx * dx + dy * dy).max(0.00001);
        for y in y0..y1 {
            for x in x0..x1 {
                let p = self.world(x, y);
                let s = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0);
                if (p.0 - a.0 - s * dx).powi(2) + (p.1 - a.1 - s * dy).powi(2) <= width * width {
                    self.blend(x, y, color, 1.0);
                }
            }
        }
    }

    pub fn polygon(&mut self, points: &[Point], color: Rgb) {
        let min = points.iter().fold((f32::INFINITY, f32::INFINITY), |a, p| {
            (a.0.min(p.0), a.1.min(p.1))
        });
        let max = points
            .iter()
            .fold((f32::NEG_INFINITY, f32::NEG_INFINITY), |a, p| {
                (a.0.max(p.0), a.1.max(p.1))
            });
        let (x0, y0, x1, y1) = self.bounds(min, max);
        for y in y0..y1 {
            for x in x0..x1 {
                let p = self.world(x, y);
                let mut inside = false;
                let mut prev = points[points.len() - 1];
                for &next in points {
                    if (next.1 > p.1) != (prev.1 > p.1)
                        && p.0 < (prev.0 - next.0) * (p.1 - next.1) / (prev.1 - next.1) + next.0
                    {
                        inside = !inside;
                    }
                    prev = next;
                }
                if inside {
                    self.blend(x, y, color, 1.0);
                }
            }
        }
    }

    pub fn star(&mut self, center: Point, radius: f32, angle: f32, color: Rgb) {
        let points: Vec<_> = (0..10)
            .map(|i| {
                let a = angle + i as f32 * TAU / 10.0;
                let r = radius * if i % 2 == 0 { 1.0 } else { 0.42 };
                (center.0 + a.cos() * r, center.1 + a.sin() * r)
            })
            .collect();
        self.polygon(&points, color);
    }

    /// Closed Catmull-Rom outline for hair, fabric, fins and wings.
    pub fn blob(&mut self, points: &[Point], color: Rgb) {
        let n = points.len();
        let mut curve = Vec::with_capacity(n * 8);
        for i in 0..n {
            let [a, b, c, d] = [
                points[(i + n - 1) % n],
                points[i],
                points[(i + 1) % n],
                points[(i + 2) % n],
            ];
            for j in 0..8 {
                let t = j as f32 / 8.0;
                let axis = |a: f32, b: f32, c: f32, d: f32| {
                    0.5 * ((2.0 * b)
                        + (-a + c) * t
                        + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t
                        + (-a + 3.0 * b - 3.0 * c + d) * t * t * t)
                };
                curve.push((axis(a.0, b.0, c.0, d.0), axis(a.1, b.1, c.1, d.1)));
            }
        }
        self.polygon(&curve, color);
    }

    pub fn curve(&mut self, points: [Point; 4], width: f32, color: Rgb) {
        let mut previous = points[0];
        for i in 1..=20 {
            let t = i as f32 / 20.0;
            let u = 1.0 - t;
            let axis = |c: usize| {
                let v = points.map(|p| if c == 0 { p.0 } else { p.1 });
                u * u * u * v[0]
                    + 3.0 * u * u * t * v[1]
                    + 3.0 * u * t * t * v[2]
                    + t * t * t * v[3]
            };
            let next = (axis(0), axis(1));
            self.line(previous, next, width, color);
            previous = next;
        }
    }

    pub fn ring(&mut self, center: Point, radius: f32, squash: f32, color: Rgb) {
        for i in 0..48 {
            let a = i as f32 * TAU / 48.0;
            let b = (i + 1) as f32 * TAU / 48.0;
            self.line(
                (
                    center.0 + a.cos() * radius,
                    center.1 + a.sin() * radius * squash,
                ),
                (
                    center.0 + b.cos() * radius,
                    center.1 + b.sin() * radius * squash,
                ),
                0.5,
                color,
            );
        }
    }

    pub fn text(&mut self, text: &str, center_x: f32, y: f32, color: Rgb) {
        let start = (center_x - (text.len() as f32 * 4.0 - 1.0) * 0.5).floor();
        for (i, ch) in text.bytes().enumerate() {
            let rows = match ch {
                b'A' => [2, 5, 7, 5, 5],
                b'B' => [6, 5, 6, 5, 6],
                b'C' => [3, 4, 4, 4, 3],
                b'D' => [6, 5, 5, 5, 6],
                b'E' => [7, 4, 6, 4, 7],
                b'F' => [7, 4, 6, 4, 4],
                b'G' => [3, 4, 5, 5, 3],
                b'H' => [5, 5, 7, 5, 5],
                b'I' => [7, 2, 2, 2, 7],
                b'J' => [1, 1, 1, 5, 2],
                b'K' => [5, 5, 6, 5, 5],
                b'L' => [4, 4, 4, 4, 7],
                b'M' => [5, 7, 7, 5, 5],
                b'N' => [5, 7, 7, 7, 5],
                b'O' => [2, 5, 5, 5, 2],
                b'P' => [6, 5, 6, 4, 4],
                b'Q' => [2, 5, 5, 3, 1],
                b'R' => [6, 5, 6, 5, 5],
                b'S' => [3, 4, 2, 1, 6],
                b'T' => [7, 2, 2, 2, 2],
                b'U' => [5, 5, 5, 5, 7],
                b'V' => [5, 5, 5, 5, 2],
                b'W' => [5, 5, 7, 7, 5],
                b'X' => [5, 5, 2, 5, 5],
                b'Y' => [5, 5, 2, 2, 2],
                b'Z' => [7, 1, 2, 4, 7],
                b'!' => [2, 2, 2, 0, 2],
                _ => [0; 5],
            };
            for (dy, bits) in rows.into_iter().enumerate() {
                for dx in 0..3 {
                    if bits & (4 >> dx) != 0 {
                        let x = start + i as f32 * 4.0 + dx as f32;
                        let y = y + dy as f32;
                        self.polygon(
                            &[(x, y), (x + 1.0, y), (x + 1.0, y + 1.0), (x, y + 1.0)],
                            color,
                        );
                    }
                }
            }
        }
    }
}
