//! A small CPU light buffer, resolved through the public sample-based Canvas API.
use std::ops::{Add, AddAssign, Mul};

use crossterm::style::Color as TerminalColor;
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
        // Keep sample colors intact; Canvas owns terminal-cell color fitting.
        for y in 0..self.height {
            for x in 0..self.width {
                canvas.set_color(
                    x as f64,
                    (self.height - 1 - y) as f64,
                    self.pixels[y * self.width + x].exposed().terminal(),
                );
            }
        }
    }
}
