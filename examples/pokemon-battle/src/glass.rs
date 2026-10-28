//! Frosted backdrop composition through the public TUI Visual/CellEffect API.
use crate::raster::{Raster, Rgb};
use crossterm::style::{Color, Colors};
use rsille::tui::prelude::{CellEffect, CellSample, VisualCtx};
use std::sync::{Arc, RwLock};

/// The same live scene used by Canvas, blurred once per frame for all HUD layers.
#[derive(Default)]
pub struct Backdrop {
    width: usize,
    height: usize,
    horizontal: Vec<Rgb>,
    blurred: Vec<Rgb>,
}

impl Backdrop {
    pub fn update(&mut self, raster: &Raster) {
        self.width = raster.width;
        self.height = raster.height;
        let (w, h) = (self.width, self.height);
        self.horizontal.resize(w * h, [0.0; 3]);
        self.blurred.resize(w * h, [0.0; 3]);
        if w == 0 || h == 0 {
            return;
        }
        // Sliding windows keep this separable blur linear in the scene size.
        for y in 0..h {
            let (mut left, mut right) = (0, 0);
            let mut sum = [0.0; 3];
            for x in 0..w {
                while right < (x + 5).min(w) {
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total += raster.pixels[y * w + right][c];
                    }
                    right += 1;
                }
                while left < x.saturating_sub(4) {
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total -= raster.pixels[y * w + left][c];
                    }
                    left += 1;
                }
                self.horizontal[y * w + x] = sum.map(|v| v / (right - left) as f32);
            }
        }
        for x in 0..w {
            let (mut top, mut bottom) = (0, 0);
            let mut sum = [0.0; 3];
            for y in 0..h {
                while bottom < (y + 7).min(h) {
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total += self.horizontal[bottom * w + x][c];
                    }
                    bottom += 1;
                }
                while top < y.saturating_sub(6) {
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total -= self.horizontal[top * w + x][c];
                    }
                    top += 1;
                }
                self.blurred[y * w + x] = sum.map(|v| v / (bottom - top) as f32);
            }
        }
    }

    pub fn cell(&self, x: usize, y: usize) -> Option<Rgb> {
        // Both scene encoders rasterize at 2x4 subpixels per terminal cell.
        let (px, py) = (x * 2, y * 4);
        if px + 1 >= self.width || py + 3 >= self.height {
            return None;
        }
        let mut rgb = [0.0; 3];
        for dy in 0..4 {
            for dx in 0..2 {
                let p = self.blurred[(py + dy) * self.width + px + dx];
                for c in 0..3 {
                    rgb[c] += p[c] / 8.0;
                }
            }
        }
        Some(rgb)
    }
}

pub type SharedBackdrop = Arc<RwLock<Backdrop>>;

pub struct Glass {
    pub backdrop: SharedBackdrop,
    pub opacity: f32,
}
impl CellEffect for Glass {
    fn apply(&self, sample: &mut CellSample, ctx: VisualCtx<'_>) {
        let x = ctx.area.x() as usize + sample.dest_x.round().max(0.0) as usize;
        let y = ctx.area.y() as usize + sample.dest_y.round().max(0.0) as usize;
        let Some(back) = self
            .backdrop
            .read()
            .ok()
            .and_then(|backdrop| backdrop.cell(x, y))
        else {
            return;
        };
        let mut colors = sample.content.style.colors.unwrap_or(Colors {
            foreground: None,
            background: None,
        });
        let tint = match colors.background {
            Some(Color::Rgb { r, g, b }) => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0],
            _ => [0.035, 0.065, 0.11],
        };
        // Keep text and native rounded borders crisp; composite only their backdrop.
        let sheen =
            (1.0 - sample.dest_y / ctx.area.height().max(1) as f64).clamp(0.0, 1.0) as f32 * 0.025;
        let rgb: [u8; 3] = std::array::from_fn(|c| {
            ((tint[c] * self.opacity + back[c] * (1.0 - self.opacity) + sheen).clamp(0.0, 1.0)
                * 255.0)
                .round() as u8
        });
        let [r, g, b] = rgb;
        colors.background = Some(Color::Rgb { r, g, b });
        sample.content.style.colors = Some(colors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blur_preserves_a_constant_field_and_spreads_small_details() {
        let mut raster = Raster::default();
        raster.begin(20, 20, 20, crate::raster::Camera::default());
        raster.pixels.fill([0.2, 0.4, 0.6]);
        let mut backdrop = Backdrop::default();
        backdrop.update(&raster);
        let rgb = backdrop.cell(3, 2).unwrap();
        for (a, b) in rgb.into_iter().zip([0.2, 0.4, 0.6]) {
            assert!((a - b).abs() < 0.00001);
        }
        raster.pixels.fill([0.0; 3]);
        raster.pixels[10 * 20 + 10] = [1.0; 3];
        backdrop.update(&raster);
        assert!(backdrop.cell(4, 2).unwrap()[0] > 0.0);
        assert!(backdrop.cell(4, 2).unwrap()[0] < 0.05);
        assert!(backdrop.cell(10, 5).is_none());
    }
}
