use std::io;

use crossterm::{
    queue,
    style::{Attributes, Colors, SetAttributes, SetColors},
};

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stylized {
    c: char,
    colors: Colors,
    attr: Option<Attributes>,
}

impl Stylized {
    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        queue!(buffer, SetColors(self.colors))?;
        if let Some(a) = self.attr {
            queue!(buffer, SetAttributes(a))?
        }
        Ok(())
    }
}
