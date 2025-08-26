use crate::{DrawErr, area::Area, buffer::Buffer, style::Stylized};

#[derive(Debug)]
pub struct Chunk<'a> {
    buffer: &'a mut Buffer,
    drawable_area: Area,
}

impl<'a> Chunk<'a> {
    pub fn new(buffer: &'a mut Buffer, drawable_area: Area) -> Result<Self, DrawErr> {
        let pos = drawable_area.pos();
        let size = drawable_area.size();
        if pos.x + size.width > buffer.size().width || pos.y + size.height > buffer.size().height {
            return Err(DrawErr);
        }

        Ok(Self {
            buffer,
            drawable_area,
        })
    }

    pub fn is_inside(&self, x: u16, y: u16) -> bool {
        self.drawable_area.is_inside(x, y)
    }

    pub fn set(&mut self, x: u16, y: u16, content: Stylized) -> Result<usize, DrawErr> {
        if x + content.width() as u16 > self.drawable_area.size().width {
            return Err(DrawErr);
        }

        if let Some(pos) = self.drawable_area.to_absolute(x, y) {
            if self.buffer.is_occupied(pos) {
                return Err(DrawErr);
            }
            self.buffer.set(pos, content)
        } else {
            Err(DrawErr)
        }
    }

    pub fn set_forced(&mut self, x: u16, y: u16, content: Stylized) -> Result<usize, DrawErr> {
        if x + content.width() as u16 > self.drawable_area.size().width {
            return Err(DrawErr);
        }

        if let Some(pos) = self.drawable_area.to_absolute(x, y) {
            self.buffer.set_forced(pos, content)
        } else {
            Err(DrawErr)
        }
    }

    pub fn area(&self) -> Area {
        self.drawable_area
    }

    pub fn shrink(
        &mut self,
        top: u16,
        bottom: u16,
        left: u16,
        right: u16,
    ) -> Result<Chunk<'_>, DrawErr> {
        let new_area = self.drawable_area.shrink(top, bottom, left, right)?;

        Chunk::new(self.buffer, new_area)
    }

    pub fn from_area(&mut self, area: Area) -> Result<Chunk<'_>, DrawErr> {
        let pos = area.pos();
        let size = area.size();
        if pos.x + size.width > self.buffer.size().width
            || pos.y + size.height > self.buffer.size().height
        {
            return Err(DrawErr);
        }

        Chunk::new(self.buffer, area)
    }
}
