use std::cmp;
use std::fmt::Write;

use crate::{
    braille::{Pixel, PixelOp},
    utils::{normalize, RsilleErr},
};

#[cfg(feature = "color")]
use crate::color::{Colored, TermColor};

/// implement this for painting on [`Canvas`](struct.Canvas.html)
pub trait Paint {
    /// paint the object on the canvas
    /// usually, you shouldn't call this by yourself
    /// just implement this and use the [`paint`](struct.Canvas.html#method.paint) in [`Canvas`](struct.Canvas.html)
    fn paint(&self, canvas: &mut Canvas, x: f64, y: f64) -> Result<(), RsilleErr>;
}

/// The basic canvas
///
///
#[derive(Debug, Clone)]
pub struct Canvas {
    width: usize,
    height: usize,

    #[cfg(not(feature = "color"))]
    pixels: Vec<Vec<Pixel>>,

    #[cfg(feature = "color")]
    pixels: Vec<Vec<Colored>>,
}

impl Canvas {
    /// make a new empty canvas
    /// the size of the canvas will auto increase
    pub fn new() -> Self {
        let pixels = Vec::new();
        let (width, height) = (0, 0);
        Self {
            pixels,
            width,
            height,
        }
    }

    /// make a new empty canvas with the size
    /// the size of the canvas also would auto increase
    pub fn with_capcity(width: usize, height: usize) -> Self {
        let vec = make_vec(width);
        let pixels = vec![vec; height];
        Self {
            pixels,
            width,
            height,
        }
    }

    /// return the (width, height) of the canvas
    pub fn get_capcity(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// paint those Paintable object on the location (x, y)
    pub fn paint(&mut self, target: &dyn Paint, x: f64, y: f64) -> Result<(), RsilleErr> {
        if x < 0.0 || y < 0.0 {
            return Err(RsilleErr::new(format!(
                "can't paint on postion {:#?}!",
                (x, y)
            )));
        }
        target.paint(self, x, y)?;
        Ok(())
    }

    /// return the string prepare to print
    pub fn frame(&mut self) -> String {
        self.get_lines().join("\n")
    }

    /// return the lines of the canvas
    pub fn get_lines(&mut self) -> Vec<String> {
        self.pixels
            .iter()
            .map(|row| {
                row.iter().fold(String::new(), |mut out, p| {
                    let _ = write!(out, "{}", *p);
                    out
                })
            })
            .collect()
    }

    /// clear the canvas
    pub fn clear(&mut self) {
        let width = self.width;
        let height = self.height;
        let vec = make_vec(width);
        self.pixels = vec![vec; height];

        // #[cfg(feature = "color")]
        // print!("{}", TermColor::Unset);
    }

    /// draw a dot on (x, y)
    /// just use the (x, y) in your object, the algorithm will find the right location
    pub fn set(&mut self, x: f64, y: f64) {
        let (row, col) = get_pos(x, y);
        self.pad_row_col(row, col);
        self.pixels[row][col].set(x, y);
    }

    /// similar to [`set`](struct.Canvas.html#method.set)
    /// but it's support color
    #[cfg(feature = "color")]
    pub fn set_colorful(&mut self, x: f64, y: f64, color: TermColor) {
        let (row, col) = get_pos(x, y);
        self.pad_row_col(row, col);
        self.pixels[row][col].set(x, y);
        self.pixels[row][col].set_color(color);
    }

    /// if the (x, y) is already set, then unset it
    /// if the (x, y) is unset, then set it
    pub fn toggle(&mut self, x: f64, y: f64) {
        let (row, col) = get_pos(x, y);
        self.pad_row_col(row, col);
        self.pixels[row][col].toggle(x, y);
    }

    /// fill ⣿ at the (x, y)
    /// the (x, y) is the real location on canvas, it's hard to use
    /// don't use it unless you know what you are doing!
    pub fn set_fill(&mut self, x: f64, y: f64) {
        let (row, col) = (y.round() as usize, x.round() as usize);
        self.pad_row_col(row, col);
        self.pixels[row][col].fill();
    }

    /// similar to [`set_fill`](struct.Canvas.html#method.set_fill), but support color
    /// don't use it unless you know what you are doing!
    #[cfg(feature = "color")]
    pub fn set_fill_colorful(&mut self, x: f64, y: f64, color: TermColor) {
        let (row, col) = (y.round() as usize, x.round() as usize);
        self.pad_row_col(row, col);
        self.pixels[row][col].fill();
        self.pixels[row][col].set_color(color);
    }

    /// draw a line on the canvas
    /// * `xy1` - the start location
    /// * `xy2` = the end location
    pub fn line(&mut self, xy1: (f64, f64), xy2: (f64, f64)) {
        let (x1, y1) = (normalize(xy1.0), normalize(xy1.1));
        let (x2, y2) = (normalize(xy2.0), normalize(xy2.1));
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

    /// draw a line on the canvas with the color
    /// * `xy1` - the start location
    /// * `xy2` = the end location
    #[cfg(feature = "color")]
    pub fn line_colorful(&mut self, xy1: (f64, f64), xy2: (f64, f64), color: TermColor) {
        let (x1, y1) = (normalize(xy1.0), normalize(xy1.1));
        let (x2, y2) = (normalize(xy2.0), normalize(xy2.1));
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
            self.set_colorful(x, y, color);
        }
    }

    // +--+    +----+    +----+
    // |  | -> |    | -> |    |
    // +--+    +----+    |    |
    //                   +----+
    fn pad_row_col(&mut self, row: usize, col: usize) {
        if self.width <= col {
            for r in &mut self.pixels {
                let pad_num = col - r.len() + 1;
                let mut vec = make_vec(pad_num);
                r.append(&mut vec);
            }
            self.width = col + 1;
        }

        if self.height <= row {
            let pad_num = row - self.height + 1;
            let vec = make_vec(self.width);
            let mut pad = vec![vec; pad_num];
            self.pixels.append(&mut pad);
            self.height = row + 1;
        }
    }
}

fn get_pos(x: f64, y: f64) -> (usize, usize) {
    (y.round() as usize / 4, x.round() as usize / 2)
}

#[cfg(not(feature = "color"))]
fn make_vec(len: usize) -> Vec<Pixel> {
    vec![Pixel::new(); len]
}

#[cfg(feature = "color")]
fn make_vec(len: usize) -> Vec<Colored> {
    vec![Colored::new(Pixel::new(), TermColor::None); len]
}
