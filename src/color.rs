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
//! println!("{}", c.render());
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

// impl fmt::Display for Colored {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}{}{}", self.color, self.pixel, Color::Unset)
//     }
// }

// /// Colors in Terminal
// ///
// /// it support 16 colors or 256 colors or rgb color
// #[derive(Debug, Clone, Copy, PartialEq)]
// pub enum Color {
//     /// the 16 colors in terminal
//     C16(Color16),
//     /// the 256 colors in terminal
//     C256(u8),
//     /// the rgb colors in terminal, may not work in some console
//     Crgb(u8, u8, u8),
//     /// no color, it won't change the color of output
//     None,
//     /// reset the color, it will change the color of output
//     Unset,
// }

// impl fmt::Display for Color {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         use Color::*;
//         match self {
//             C16(c16) => write!(f, "\x1B[{}m", *c16 as u8),
//             C256(c256) => write!(f, "\x1B[38;5;{}m", *c256),
//             Crgb(r, g, b) => write!(f, "\x1B[38;2;{};{};{}m", *r, *g, *b),
//             None => write!(f, ""),
//             Unset => write!(f, "\x1B[m"),
//         }
//     }
// }

// /// The 16 colors in Terminal
// #[rustfmt::skip]
// #[allow(missing_docs)]
// #[derive(Debug, Clone, Copy, PartialEq)]
// pub enum Color16 {
//     Black        = 30,
//     Red          = 31,
//     Green        = 32,
//     Yellow       = 33,
//     Blue         = 34,
//     Purple       = 35,
//     Cyan         = 36,
//     White        = 37,
//     BrightBlack  = 90,
//     BrightRed    = 91,
//     BrightGreen  = 92,
//     BrightYellow = 93,
//     BrightBlue   = 94,
//     BrightPurple = 95,
//     BrightCyan   = 96,
//     BrightWhite  = 97,
// }
