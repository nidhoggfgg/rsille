use crate::{
    DrawErr,
    area::{Area, Size, Position},
    style::Stylized,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Buffer {
    size: Size,
    content: Vec<Stylized>,
}

// every position in the buffer is absolute
impl Buffer {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            content: vec![Stylized::space(); (size.width * size.height) as usize],
        }
    }

    pub fn index(&self, pos: Position) -> Option<usize> {
        if pos.x < self.size.width && pos.y < self.size.height {
            Some((pos.y * self.size.width + pos.x) as usize)
        } else {
            None
        }
    }

    pub fn get(&self, pos: Position) -> Option<&Stylized> {
        let i = self.index_unchecked(pos);
        if i < self.content.len() {
            Some(&self.content[i])
        } else {
            None
        }
    }

    pub fn set(&mut self, pos: Position, content: Stylized) -> Result<(), DrawErr> {
        let i = self.index_unchecked(pos);
        if i >= self.content.len() {
            return Err(DrawErr);
        }

        self.content[i] = content;
        Ok(())
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn content(&self) -> &[Stylized] {
        &self.content
    }

    pub fn index_unchecked(&self, pos: Position) -> usize {
        (pos.y * self.size.width + pos.x) as usize
    }
}

#[derive(Debug)]
pub struct Chunk<'a> {
    buffer: &'a mut Buffer,
    drawable_area: Area,
}

impl<'a> Chunk<'a> {
    pub fn new(buffer: &'a mut Buffer, drawable_area: Area) -> Result<Self, DrawErr> {
        if drawable_area.pos.x + drawable_area.size.width > buffer.size.width ||
           drawable_area.pos.y + drawable_area.size.height > buffer.size.height {
            return Err(DrawErr);
        }

        Ok(Self { buffer, drawable_area })
    }

    pub fn is_inside(&self, x: u16, y: u16) -> bool {
        self.drawable_area.is_inside(x, y)
    }

    pub fn set(&mut self, x: u16, y: u16, content: Stylized) -> Result<(), DrawErr> {
        if let Some(pos) = self.drawable_area.to_absolute(x, y) {
            self.buffer.set(pos, content)
        } else {
            Err(DrawErr)
        }
    }

    pub fn area(&self) -> Area {
        self.drawable_area
    }

    pub fn shrink(&mut self, top: u16, bottom: u16, left: u16, right: u16) -> Result<Chunk<'_>, DrawErr> {
        let new_area = self.drawable_area.shrink(top, bottom, left, right)?;

        Chunk::new(self.buffer, new_area)
    }
}
