use log::trace;

use crate::render::{
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
        let content_width = content.width_with(self.text_metrics(), x as usize) as u16;
        if x as usize + content_width as usize > self.area.size().width as usize {
            trace!(
                target: "render::chunk",
                "content too wide at ({}, {}): width={}, available={}",
                x, y, content_width, self.area.size().width.saturating_sub(x)
            );
            return Err(DrawErr::content_too_wide(
                content_width,
                self.area.size().width,
                x,
            ));
        }

        if content.as_char() == Some('\t') {
            for offset in 0..content_width {
                self.set(x + offset, y, Stylized::new(' ', content.style))?;
            }
            return Ok(content_width as usize);
        }

        if let Some(pos) = self.area.to_absolute(x, y) {
            if self.buffer.is_occupied(pos) {
                trace!(
                    target: "render::chunk",
                    "position occupied at ({}, {})",
                    x, y
                );
                return Err(DrawErr::position_occupied(pos));
            }
            self.buffer.set(pos, content)
        } else {
            trace!(
                target: "render::chunk",
                "out of bounds at ({}, {}), area size: {}x{}",
                x, y, self.area.size().width, self.area.size().height
            );
            Err(DrawErr::out_of_bounds((x, y).into(), self.area.size()))
        }
    }

    pub fn set_forced(&mut self, x: u16, y: u16, content: Stylized) -> Result<usize, DrawErr> {
        let content_width = content.width_with(self.text_metrics(), x as usize) as u16;
        if x as usize + content_width as usize > self.area.size().width as usize {
            trace!(
                target: "render::chunk",
                "forced set: content too wide at ({}, {}): width={}, available={}",
                x, y, content_width, self.area.size().width.saturating_sub(x)
            );
            return Err(DrawErr::content_too_wide(
                content_width,
                self.area.size().width,
                x,
            ));
        }

        if content.as_char() == Some('\t') {
            for offset in 0..content_width {
                self.set_forced(x + offset, y, Stylized::new(' ', content.style))?;
            }
            return Ok(content_width as usize);
        }

        if let Some(pos) = self.area.to_absolute(x, y) {
            self.buffer.overwrite(pos, content)
        } else {
            trace!(
                target: "render::chunk",
                "forced set: out of bounds at ({}, {}), area size: {}x{}",
                x, y, self.area.size().width, self.area.size().height
            );
            Err(DrawErr::out_of_bounds((x, y).into(), self.area.size()))
        }
    }

    pub fn text_metrics(&self) -> crate::render::text::TextMetrics {
        self.buffer.text_metrics()
    }

    /// Read the current render target, including cells outside this chunk.
    ///
    /// Buffer coordinates are absolute. This is useful for sampling previously
    /// painted layers; writes remain restricted to the chunk's area.
    pub fn buffer(&self) -> &Buffer {
        self.buffer
    }

    pub fn set_text_metrics(&mut self, metrics: crate::render::text::TextMetrics) {
        self.buffer.set_text_metrics(metrics);
    }

    /// Paint rich text, preserving graphemes crossing span boundaries.
    pub fn set_line(
        &mut self,
        x: u16,
        y: u16,
        line: &crate::render::text::Line<'_>,
    ) -> Result<(), DrawErr> {
        let mut column = 0usize;
        for cell in line.cells() {
            let width = cell.width_with(self.text_metrics(), column);
            if x as usize + column + width > self.area.width() as usize {
                break;
            }
            if width == 0 {
                continue;
            }
            if cell.as_char() == Some('\t') {
                for offset in 0..width {
                    self.set_forced(
                        x + (column + offset) as u16,
                        y,
                        Stylized::new(' ', cell.style),
                    )?;
                }
            } else {
                self.set_forced(x + column as u16, y, cell)?;
            }
            column += width;
        }
        Ok(())
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

    /// Create a child chunk clipped to this chunk's bounds.
    ///
    /// The supplied area is in absolute terminal coordinates, matching
    /// [`from_area`](Self::from_area). If the area has no overlap with the
    /// current chunk, the closure is not called.
    pub fn with_clip<R>(
        &mut self,
        area: Area,
        render: impl FnOnce(&mut Chunk<'_>) -> R,
    ) -> Result<Option<R>, DrawErr> {
        let Some(clipped) = area.clamp_to(&self.area) else {
            return Ok(None);
        };

        let mut chunk = Chunk::new(self.buffer, clipped)?;
        Ok(Some(render(&mut chunk)))
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
        use crate::render::text::UnicodeSegmentation;
        let mut column = 0usize;
        let metrics = self.text_metrics();
        for g in string.graphemes(true) {
            let width = metrics.grapheme_width(g, column);
            if x as usize + column + width > self.area.width() as usize {
                break;
            }
            if width == 0 {
                continue;
            }
            if g == "\t" {
                for offset in 0..width {
                    self.set_forced(x + (column + offset) as u16, y, Stylized::new(' ', style))?;
                }
            } else {
                self.set_forced(x + column as u16, y, Stylized::grapheme(g, style))?;
            }
            column += width;
        }
        Ok(())
    }

    /// Paint rich text with explicit line boundaries and clipping.
    pub fn set_text(
        &mut self,
        x: u16,
        y: u16,
        text: &crate::render::text::Text<'_>,
    ) -> Result<(), DrawErr> {
        for (row, line) in text.lines.iter().enumerate() {
            let row = y as usize + row;
            if row >= self.area.height() as usize {
                break;
            }
            self.set_line(x, row as u16, line)?;
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
            let mut dx = 0usize;
            while dx < width as usize {
                let stylized = Stylized::new(ch, style);
                let columns = stylized.width_with(self.text_metrics(), x as usize + dx);
                if columns == 0 || dx + columns > width as usize {
                    break;
                }
                let _ =
                    self.set_forced(x.saturating_add(dx as u16), y.saturating_add(dy), stylized);
                dx += columns;
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
