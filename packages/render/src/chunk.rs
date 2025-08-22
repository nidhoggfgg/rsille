use std::slice::Chunks;

use crate::{
    DrawErr,
    area::{Area, Size},
    style::Stylized,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Buffer {
    pub size: Size,
    pub content: Vec<Stylized>,
}

impl Buffer {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            content: vec![Stylized::space(); (size.width * size.height) as usize],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Chunk<'a> {
    buffer: &'a mut Buffer,
    drawable_area: Area,
}

impl<'a> Chunk<'a> {
    pub fn new(buffer: &'a mut Buffer, drawable_area: Area) -> Self {
        Self {
            buffer,
            drawable_area,
        }
    }

    pub fn shrink(self, size: u16) -> Result<Chunk<'a>, DrawErr> {
        let new_drawable_area = self.drawable_area.shrink(size)?;

        Ok(Self {
            buffer: self.buffer,
            drawable_area: new_drawable_area,
        })
    }

    pub fn set_drawable_area(&mut self, area: Area) -> Result<(), DrawErr> {
        if area.pos.x + area.size.width > self.buffer.size.width
            || area.pos.y + area.size.height > self.buffer.size.height
        {
            return Err(DrawErr);
        }

        self.drawable_area = area;

        Ok(())
    }

    pub fn lines(&self) -> Chunks<'_, Stylized> {
        self.buffer.content.chunks(self.buffer.size.width as usize)
    }

    pub fn index(&self, x: u16, y: u16) -> Option<usize> {
        if self.buffer.size.width > x && self.buffer.size.height > y {
            Some((y * self.buffer.size.width + x) as usize)
        } else {
            None
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Stylized> {
        if let Some(i) = self.index(x, y) {
            Some(&self.buffer.content[i])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Stylized> {
        let drawable_width = self.drawable_area.size.width;
        let drawable_height = self.drawable_area.size.height;

        if x >= drawable_width || y >= drawable_height {
            return None;
        }

        if let Some(i) = self.index(self.drawable_area.pos.x + x, self.drawable_area.pos.y + y) {
            return Some(&mut self.buffer.content[i]);
        }

        None
    }
}
