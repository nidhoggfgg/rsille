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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Colored {
    pixel: Pixel,
    color: Colors,
}

#[allow(unused)]
impl Colored {
    pub fn new() -> Self {
        Self {
            pixel: Pixel::new(),
            color: Colors {
                foreground: None,
                background: None,
            },
        }
    }

    pub fn set_foregound_color(&mut self, color: Color) -> &mut Self {
        self.color.foreground = Some(color);
        self
    }

    pub fn set_background_color(&mut self, color: Color) -> &mut Self {
        self.color.background = Some(color);
        self
    }

    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        if self.color.foreground.is_none() && self.color.background.is_none() {
            queue!(buffer, Print(format!("{}", self.pixel)),)
        } else {
            queue!(
                buffer,
                SetColors(self.color),
                Print(format!("{}", self.pixel)),
                ResetColor
            )
        }
    }
}

impl<T> PixelOp<T> for Colored
where
    T: Into<f64> + Copy,
{
    fn set(&mut self, x: T, y: T) {
        self.pixel.set(x, y);
    }

    fn unset(&mut self, x: T, y: T) {
        self.pixel.unset(x, y);
    }

    fn toggle(&mut self, x: T, y: T) {
        self.pixel.toggle(x, y);
    }
}
