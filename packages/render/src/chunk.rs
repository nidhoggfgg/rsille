use crate::{
    area::Area,
    buffer::Buffer,
    style::{Style, Stylized},
    DrawErr,
};

#[derive(Debug)]
pub struct Chunk<'a> {
    buffer: &'a mut Buffer,
    area: Area,
}

impl<'a> Chunk<'a> {
    pub fn new(buffer: &'a mut Buffer, area: Area) -> Result<Self, DrawErr> {
        if buffer.size().less_any(area.real_size()) {
            return Err(DrawErr::invalid_area(area.real_size(), buffer.size()));
        }

        Ok(Self { buffer, area })
    }

    pub fn is_inside(&self, x: u16, y: u16) -> bool {
        self.area.is_inside(x, y)
    }

    pub fn set(&mut self, x: u16, y: u16, content: Stylized) -> Result<usize, DrawErr> {
        let content_width = content.width() as u16;
        if x + content_width > self.area.size().width {
            return Err(DrawErr::content_too_wide(
                content_width,
                self.area.size().width,
                x,
            ));
        }

        if let Some(pos) = self.area.to_absolute(x, y) {
            if self.buffer.is_occupied(pos) {
                return Err(DrawErr::position_occupied(pos));
            }
            self.buffer.set(pos, content)
        } else {
            Err(DrawErr::out_of_bounds((x, y).into(), self.area.size()))
        }
    }

    pub fn set_forced(&mut self, x: u16, y: u16, content: Stylized) -> Result<usize, DrawErr> {
        let content_width = content.width() as u16;
        if x + content_width > self.area.size().width {
            return Err(DrawErr::content_too_wide(
                content_width,
                self.area.size().width,
                x,
            ));
        }

        if let Some(pos) = self.area.to_absolute(x, y) {
            self.buffer.overwrite(pos, content)
        } else {
            Err(DrawErr::out_of_bounds((x, y).into(), self.area.size()))
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

    /// Set a string at the specified position with style
    /// This is a convenience method for TUI-style rendering
    pub fn set_string(
        &mut self,
        x: u16,
        y: u16,
        string: &str,
        style: Style,
    ) -> Result<(), DrawErr> {
        let mut current_x = x;
        for ch in string.chars() {
            if current_x >= self.area.size().width {
                break; // Stop if we exceed the chunk width
            }
            let stylized = Stylized::new(ch, style);
            match self.set_forced(current_x, y, stylized) {
                Ok(width) => current_x += width as u16,
                Err(_) => break, // Stop on error
            }
        }
        Ok(())
    }

    /// Fill a rectangular area with a character
    pub fn fill(
        &mut self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        ch: char,
        style: Style,
    ) -> Result<(), DrawErr> {
        for dy in 0..height {
            for dx in 0..width {
                let stylized = Stylized::new(ch, style);
                let _ = self.set_forced(x + dx, y + dy, stylized);
            }
        }
        Ok(())
    }

    /// Set a single character at the specified position
    pub fn set_char(&mut self, x: u16, y: u16, ch: char, style: Style) -> Result<(), DrawErr> {
        let stylized = Stylized::new(ch, style);
        self.set_forced(x, y, stylized)?;
        Ok(())
    }
}
