//! A small CPU light buffer, resolved through the public braille Canvas API.
use std::ops::{Add, AddAssign, Mul};

use crossterm::style::{Color as TerminalColor, Colors};
use rsille::{canvas::Canvas, tui::prelude::Color};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Light(pub f64, pub f64, pub f64);

impl Light {
    pub const WHITE: Self = Self(1.0, 0.96, 0.88);

    pub fn from_color(color: Color) -> Self {
        match color {
            Color::Rgb(r, g, b) => Self(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0),
            _ => Self::WHITE,
        }
    }

    pub fn emission(color: Color) -> Self {
        let rgb = Self::from_color(color);
        let neutral = rgb.0.min(rgb.1).min(rgb.2) * 0.8;
        // Emissive pigments need stronger chroma than the UI's readable pastels.
        Self(rgb.0 - neutral, rgb.1 - neutral, rgb.2 - neutral) * 2.0
    }

    pub fn mix(self, other: Self, amount: f64) -> Self {
        self * (1.0 - amount) + other * amount
    }

    fn energy(self) -> f64 {
        self.0 * 0.2126 + self.1 * 0.7152 + self.2 * 0.0722
    }

    fn distance(self, other: Self) -> f64 {
        (self.0 - other.0).powi(2) + (self.1 - other.1).powi(2) + (self.2 - other.2).powi(2)
    }

    fn exposed(self) -> Self {
        // A soft shoulder preserves bright cores without clipping every hue to white.
        let map = |v: f64| 1.0 - (-v.max(0.0)).exp();
        Self(map(self.0), map(self.1), map(self.2))
    }

    fn terminal(self) -> TerminalColor {
        TerminalColor::Rgb {
            r: (self.0.clamp(0.0, 1.0) * 255.0).round() as u8,
            g: (self.1.clamp(0.0, 1.0) * 255.0).round() as u8,
            b: (self.2.clamp(0.0, 1.0) * 255.0).round() as u8,
        }
    }
}

impl Add for Light {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

impl AddAssign for Light {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Mul<f64> for Light {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self(self.0 * rhs, self.1 * rhs, self.2 * rhs)
    }
}

pub(crate) struct Raster {
    pub width: usize,
    pub height: usize,
    pixels: Vec<Light>,
}

impl Raster {
    pub fn new(width: usize, height: usize, background: Color) -> Self {
        Self {
            width,
            height,
            pixels: vec![Light::from_color(background); width * height],
        }
    }

    pub fn add(&mut self, x: usize, y: usize, light: Light) {
        self.pixels[y * self.width + x] += light;
    }

    pub fn set(&mut self, x: usize, y: usize, light: Light) {
        self.pixels[y * self.width + x] = light;
    }

    pub fn glow(&mut self, x: f64, y: f64, radius: f64, light: Light) {
        let reach = radius * 2.5;
        let x0 = (x - reach).floor().max(0.0) as usize;
        let x1 = ((x + reach).ceil().max(0.0) as usize).min(self.width);
        let y0 = (y - reach).floor().max(0.0) as usize;
        let y1 = ((y + reach).ceil().max(0.0) as usize).min(self.height);
        for py in y0..y1 {
            for px in x0..x1 {
                let d = ((px as f64 - x).powi(2) + (py as f64 - y).powi(2)) / (radius * radius);
                self.add(px, py, light * (-d * 1.5).exp());
            }
        }
    }

    /// Clip before stepping: near-plane particles can project far beyond the viewport.
    pub fn streak(&mut self, start: (f64, f64), end: (f64, f64), light: Light) {
        let (dx, dy) = (end.0 - start.0, end.1 - start.1);
        let (mut enter, mut leave): (f64, f64) = (0.0, 1.0);
        for (p, q) in [
            (-dx, start.0 + 1.0),
            (dx, self.width as f64 - start.0),
            (-dy, start.1 + 1.0),
            (dy, self.height as f64 - start.1),
        ] {
            if p.abs() < 1e-9 {
                if q < 0.0 {
                    return;
                }
            } else if p < 0.0 {
                enter = enter.max(q / p);
            } else {
                leave = leave.min(q / p);
            }
        }
        if enter > leave {
            return;
        }
        let steps = ((dx.abs().max(dy.abs()) * (leave - enter)).ceil() as usize).max(1);
        for step in 0..=steps {
            let t = enter + (leave - enter) * step as f64 / steps as f64;
            self.glow(
                start.0 + dx * t,
                start.1 + dy * t,
                0.65,
                light * (0.2 + t * 0.8),
            );
        }
    }

    pub fn bloom(&mut self) {
        // Separable five-tap blur of highlights only; dark skies stay dark.
        let mut horizontal = vec![Light::default(); self.pixels.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                for offset in -2_isize..=2 {
                    let px = (x as isize + offset).clamp(0, self.width as isize - 1) as usize;
                    let source = self.pixels[y * self.width + px];
                    let excess = (source.energy() - 0.55).max(0.0);
                    horizontal[y * self.width + x] += source * (excess / (1.0 + excess) * 0.2);
                }
            }
        }
        for y in 0..self.height {
            for x in 0..self.width {
                let mut glow = Light::default();
                for offset in -2_isize..=2 {
                    let py = (y as isize + offset).clamp(0, self.height as isize - 1) as usize;
                    glow += horizontal[py * self.width + x] * 0.13;
                }
                self.add(x, y, glow);
            }
        }
    }

    pub fn resolve(&self, canvas: &mut Canvas) {
        // A braille cell has one foreground AND one background. Fit both to its
        // eight samples, instead of letting the last dot overwrite every color.
        for cy in 0..self.height / 4 {
            for cx in 0..self.width / 2 {
                let samples: [Light; 8] = std::array::from_fn(|i| {
                    self.pixels[(cy * 4 + i / 2) * self.width + cx * 2 + i % 2].exposed()
                });
                let mut low = samples[0];
                let mut high = samples[0];
                for sample in samples {
                    if sample.energy() < low.energy() {
                        low = sample;
                    }
                    if sample.energy() > high.energy() {
                        high = sample;
                    }
                }
                for _ in 0..2 {
                    let (mut dark, mut bright) = (Light::default(), Light::default());
                    let (mut dark_count, mut bright_count) = (0, 0);
                    for sample in samples {
                        if sample.distance(low) < sample.distance(high) {
                            dark += sample;
                            dark_count += 1;
                        } else {
                            bright += sample;
                            bright_count += 1;
                        }
                    }
                    if dark_count > 0 {
                        low = dark * (1.0 / dark_count as f64);
                    }
                    if bright_count > 0 {
                        high = bright * (1.0 / bright_count as f64);
                    }
                }
                let colors = Colors {
                    foreground: Some(high.terminal()),
                    background: Some(low.terminal()),
                };
                for (i, sample) in samples.into_iter().enumerate() {
                    if sample.distance(high) <= sample.distance(low) {
                        canvas.set_colorful(
                            (cx * 2 + i % 2) as f64,
                            (self.height - 1 - (cy * 4 + i / 2)) as f64,
                            colors,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsille::render::{area::Area, buffer::Buffer, chunk::Chunk, Draw};

    fn render(image: &Raster) -> Buffer {
        let size = ((image.width / 2) as u16, (image.height / 4) as u16).into();
        let mut canvas = Canvas::new();
        canvas.set_bound(
            (0, image.width as i32 / 2 - 1),
            (0, image.height as i32 / 4 - 1),
        );
        image.resolve(&mut canvas);
        let mut buffer = Buffer::new(size);
        canvas
            .draw(Chunk::new(&mut buffer, Area::new((0, 0).into(), size)).unwrap())
            .unwrap();
        buffer
    }

    #[test]
    fn bright_dot_keeps_its_color_position_and_dark_background() {
        let mut image = Raster::new(4, 8, Color::Rgb(0, 0, 0));
        image.set(0, 0, Light(5.0, 0.0, 0.0));
        image.set(3, 7, Light(0.0, 5.0, 0.0));
        let buffer = render(&image);
        assert_eq!(buffer.content()[0].content.as_char(), Some('\u{2801}'));
        assert_eq!(buffer.content()[3].content.as_char(), Some('\u{2880}'));
        let colors = buffer.content()[0].content.style.colors.unwrap();
        assert_eq!(
            colors.background,
            Some(TerminalColor::Rgb { r: 0, g: 0, b: 0 })
        );
        assert!(matches!(
            colors.foreground,
            Some(TerminalColor::Rgb {
                r: 250..=255,
                g: 0,
                b: 0
            })
        ));
    }

    #[test]
    fn clipping_handles_near_plane_streaks_and_rejects_offscreen_lines() {
        let mut image = Raster::new(8, 8, Color::Rgb(0, 0, 0));
        image.streak((-1e9, 4.0), (1e9, 4.0), Light::WHITE);
        assert!(image.pixels[4 * 8..5 * 8].iter().all(|p| p.energy() > 0.0));
        let before = image.pixels.clone();
        image.streak((-1e9, -1e9), (-10.0, -10.0), Light::WHITE);
        assert!(image
            .pixels
            .iter()
            .zip(before)
            .all(|(a, b)| a.distance(b) == 0.0));
    }
}
