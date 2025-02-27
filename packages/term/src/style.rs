use std::io;

use crossterm::{
    queue,
    style::{Attributes, Colors, Print, SetAttributes, SetColors},
};
use unicode_width::UnicodeWidthChar;

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stylized {
    c: Option<char>,
    colors: Option<Colors>,
    attr: Option<Attributes>,
}

impl Stylized {
    pub fn new(c: char, colors: Option<Colors>, attr: Option<Attributes>) -> Self {
        Self {
            c: Some(c),
            colors,
            attr,
        }
    }

    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        if let Some(c) = self.c {
            if let Some(c) = self.colors {
                queue!(buffer, SetColors(c))?
            }
            if let Some(a) = self.attr {
                queue!(buffer, SetAttributes(a))?
            }
            queue!(buffer, Print(c))?;
        }
        Ok(())
    }

    pub fn space() -> Self {
        Self {
            c: Some(' '),
            colors: None,
            attr: None,
        }
    }

    pub fn nop() -> Self {
        Self {
            c: None,
            colors: None,
            attr: None,
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        if let Some(c) = self.c {
            return c.width().unwrap_or(0);
        }
        0
    }

    #[inline]
    pub fn width_cjk(&self) -> usize {
        if let Some(c) = self.c {
            return c.width_cjk().unwrap_or(0);
        }
        0
    }
}
