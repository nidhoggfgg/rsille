use std::cmp;

use crate::{braille::Pixel, utils::normalize};

pub trait Paint {
    fn paint(&self, canvas: &mut Canvas, x: f64, y: f64);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    pixels: Vec<Vec<Pixel>>,
    width: usize,
    height: usize,
}

impl Canvas {
    pub fn new() -> Self {
        let pixels = Vec::new();
        let width = 0;
        let height = 0;
        Self {
            pixels,
            width,
            height,
        }
    }

    pub fn with_capcity(width: usize, height: usize) -> Self {
        let pixels = vec![vec![Pixel::zero(); width]; height];
        Self {
            pixels,
            width,
            height,
        }
    }

    pub fn paint(&mut self, target: &dyn Paint, x: f64, y: f64) {
        target.paint(self, x, y);
    }

    pub fn frame(&mut self) -> String {
        self.lines().join("\n")
    }

    pub fn lines(&mut self) -> Vec<String> {
        self.pixels
            .iter()
            .map(|row| row.iter().map(|p| p.get()).collect::<String>())
            .collect()
    }

    pub fn clear(&mut self) {
        let width = self.width;
        let height = self.height;
        self.pixels = vec![vec![Pixel::zero(); width]; height];
    }

    pub fn set(&mut self, x: f64, y: f64) {
        let (row, col) = get_pos(x, y);
        self.pad_row_col(row, col);
        self.pixels[row][col].set(x, y);
    }

    pub fn toggle(&mut self, x: f64, y: f64) {
        let (row, col) = get_pos(x, y);
        self.pad_row_col(row, col);
        self.pixels[row][col].toggle(x, y);
    }

    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        let (x1, y1) = (normalize(x1), normalize(y1));
        let (x2, y2) = (normalize(x2), normalize(y2));
        let d = |v1, v2| {
            if v1 <= v2 {
                (v2 - v1, 1.0)
            } else {
                (v1 - v2, -1.0)
            }
        };

        let (xdiff, xdir) = d(x1, x2);
        let (ydiff, ydif) = d(y1, y2);
        let r = cmp::max(xdiff, ydiff);

        for i in 0..=r {
            let r = r as f64;
            let i = i as f64;
            let (xd, yd) = (xdiff as f64, ydiff as f64);
            let x = x1 as f64 + i * xd / r * xdir;
            let y = y1 as f64 + i * yd / r * ydif;
            self.set(x, y);
        }
    }

    // +--+    +----+    +----+
    // |  | -> |    | -> |    |
    // +--+    +----+    |    |
    //                  +----+
    fn pad_row_col(&mut self, row: usize, col: usize) {
        if self.width <= col {
            for r in &mut self.pixels {
                let pad_num = col - r.len() + 1;
                r.append(&mut vec![Pixel::zero(); pad_num]);
            }
            self.width = col + 1;
        }

        if self.height <= row {
            let pad_num = row - self.height + 1;
            let mut pad = vec![vec![Pixel::zero(); self.width]; pad_num];
            self.pixels.append(&mut pad);
            self.height = row + 1;
        }
    }
}

fn get_pos(x: f64, y: f64) -> (usize, usize) {
    (y.round() as usize / 4, x.round() as usize / 2)
}
