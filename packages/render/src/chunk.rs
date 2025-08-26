use crate::{DrawErr, area::Area, buffer::Buffer, style::Stylized};

#[derive(Debug)]
pub struct Chunk<'a> {
    buffer: &'a mut Buffer,
    area: Area,
}

impl<'a> Chunk<'a> {
    pub fn new(buffer: &'a mut Buffer, area: Area) -> Result<Self, DrawErr> {
        if buffer.size().less_any(area.real_size()) {
            return Err(DrawErr);
        }

        Ok(Self { buffer, area })
    }

    pub fn is_inside(&self, x: u16, y: u16) -> bool {
        self.area.is_inside(x, y)
    }

    pub fn set(&mut self, x: u16, y: u16, content: Stylized) -> Result<usize, DrawErr> {
        if x + content.width() as u16 > self.area.size().width {
            return Err(DrawErr);
        }

        if let Some(pos) = self.area.to_absolute(x, y) {
            if self.buffer.is_occupied(pos) {
                return Err(DrawErr);
            }
            self.buffer.set(pos, content)
        } else {
            Err(DrawErr)
        }
    }

    pub fn set_forced(&mut self, x: u16, y: u16, content: Stylized) -> Result<usize, DrawErr> {
        if x + content.width() as u16 > self.area.size().width {
            return Err(DrawErr);
        }

        if let Some(pos) = self.area.to_absolute(x, y) {
            self.buffer.set_forced(pos, content)
        } else {
            Err(DrawErr)
        }
    }

    pub fn area(&self) -> Area {
        self.area
    }

    pub fn shrink(
        &mut self,
        top: u16,
        bottom: u16,
        left: u16,
        right: u16,
    ) -> Result<Chunk<'_>, DrawErr> {
        let new_area = self.area.shrink(top, bottom, left, right)?;

        Chunk::new(self.buffer, new_area)
    }

    pub fn from_area(&mut self, area: Area) -> Result<Chunk<'_>, DrawErr> {
        Chunk::new(self.buffer, area)
    }
}
