use std::io;

use crate::braille::{Pixel, PixelOp};

pub use crossterm::style::Color;
use crossterm::{
    queue,
    style::{Colors, Print, ResetColor, SetColors},
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colored {
    pixel: Pixel,
    color: Colors,
}

impl Colored {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            pixel: Pixel::new(),
            color: Colors {
                foreground: None,
                background: None,
            },
        }
    }

    #[inline]
    pub fn set_foregound_color(&mut self, color: Color) -> &mut Self {
        self.color.foreground = Some(color);
        self
    }

    #[inline]
    pub fn set_background_color(&mut self, color: Color) -> &mut Self {
        self.color.background = Some(color);
        self
    }

    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        if self.color.foreground.is_none() && self.color.background.is_none() {
            queue!(buffer, Print(self.pixel))
        } else {
            queue!(buffer, SetColors(self.color), Print(self.pixel), ResetColor)
        }
    }
}

impl PixelOp for Colored {
    fn set(&mut self, x: i32, y: i32) -> &mut Self {
        self.pixel.set(x, y);
        self
    }

    fn unset(&mut self, x: i32, y: i32) -> &mut Self {
        self.pixel.unset(x, y);
        self
    }

    fn toggle(&mut self, x: i32, y: i32) -> &mut Self {
        self.pixel.toggle(x, y);
        self
    }
}

impl Default for Colored {
    fn default() -> Self {
        Self::new()
    }
}
