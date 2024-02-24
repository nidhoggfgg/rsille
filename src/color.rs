//! Colors in Terminal
//!
//! ## Example
//!
//! draw a red star
//! ```
//! use rsille::{color::Color, extra::Turtle, Canvas};
//! let mut c = Canvas::new();
//! let mut t = Turtle::new();
//! t.color(Color::Red);
//! for _ in 0..5 {
//!    t.forward(30.0);
//!    t.right(144.0);
//! }
//! c.paint(&t, 0.0, 15.0).unwrap();
//! c.print();
//! ```

use std::io;

use crate::braille::{Pixel, PixelOp};

pub use crossterm::style::Color;
use crossterm::{
    queue,
    style::{Colors, Print, ResetColor, SetColors},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Colored {
    pixel: Pixel,
    color: Colors,
}

#[allow(unused)]
impl Colored {
    pub(crate) fn new() -> Self {
        Self {
            pixel: Pixel::new(),
            color: Colors {
                foreground: None,
                background: None,
            },
        }
    }

    pub(crate) fn set_foregound_color(&mut self, color: Color) {
        self.color.foreground = Some(color);
    }

    pub(crate) fn set_background_color(&mut self, color: Color) {
        self.color.background = Some(color);
    }

    pub(crate) fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        queue!(
            buffer,
            SetColors(self.color),
            Print(format!("{}", self.pixel)),
            ResetColor
        )
    }
}

impl PixelOp for Colored {
    fn set(&mut self, x: f64, y: f64) {
        self.pixel.set(x, y);
    }

    fn unset(&mut self, x: f64, y: f64) {
        self.pixel.unset(x, y);
    }

    fn toggle(&mut self, x: f64, y: f64) {
        self.pixel.toggle(x, y);
    }

    fn fill(&mut self) {
        self.pixel.fill();
    }
}
