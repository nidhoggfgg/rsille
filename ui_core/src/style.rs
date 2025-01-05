use std::io;

use terminal::crossterm::{
    queue,
    style::{Attributes, Colors, Print, SetAttributes, SetColors},
};

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stylized {
    c: char,
    colors: Option<Colors>,
    attr: Option<Attributes>,
}

impl Stylized {
    pub fn new(c: char, colors: Option<Colors>, attr: Option<Attributes>) -> Self {
        Self { c, colors, attr }
    }

    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        if let Some(c) = self.colors {
            queue!(buffer, SetColors(c))?
        }
        if let Some(a) = self.attr {
            queue!(buffer, SetAttributes(a))?
        }
        queue!(buffer, Print(self.c))?;
        Ok(())
    }

    pub fn space() -> Self {
        Self {
            c: ' ',
            colors: None,
            attr: None,
        }
    }
}
