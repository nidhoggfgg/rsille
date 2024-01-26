use core::fmt;

use crate::braille::{Pixel, PixelOp};

#[derive(Debug, Clone, Copy)]
pub struct Colored {
    pixel: Pixel,
    color: TermColor,
}

impl Colored {
    pub fn new(pixel: Pixel, color: TermColor) -> Self {
        Self { pixel, color }
    }

    pub fn set_color(&mut self, color: TermColor) {
        self.color = color;
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
}

impl fmt::Display for Colored {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.color, self.pixel)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TermColor {
    C16(Color16),
    C256(u8),
    Crgb(u8, u8, u8),
    None,
    Unset,
}

impl fmt::Display for TermColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TermColor::*;
        match self {
            C16(c16) => write!(f, "\x1B[{}m", *c16 as u8),
            C256(c256) => write!(f, "\x1B[38;5;{}m", *c256),
            Crgb(r, g, b) => write!(f, "\x1B[38;2;{};{};{}m", *r, *g, *b),
            None => write!(f, ""),
            Unset => write!(f, "\x1B[m"),
        }
    }
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy)]
pub enum Color16 {
    Black        = 30,
    Red          = 31,
    Green        = 32,
    Yellow       = 33,
    Blue         = 34,
    Purple       = 35,
    Cyan         = 36,
    White        = 37,
    BrightBlack  = 90,
    BrightRed    = 91,
    BrightGreen  = 92,
    BrightYellow = 93,
    BrightBlue   = 94,
    BrightPurple = 95,
    BrightCyan   = 96,
    BrightWhite  = 97,
}
