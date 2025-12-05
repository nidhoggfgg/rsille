use crossterm::{
    cursor::{MoveTo, MoveToNextLine, MoveToPreviousLine},
    style::{Attribute, Print, ResetColor, SetAttribute},
    terminal::{Clear, ClearType},
};
use log::debug;

use crate::queue_with_log;

use crate::{
    area::{Position, Size},
    buffer::Buffer,
    chunk::Chunk,
    Builder, Draw, DrawErr, DrawUpdate,
};

#[derive(Debug, Clone, Default)]
pub struct Render<W, T> {
    pub(crate) pos: Position,
    pub(crate) buffer: Buffer,
    pub(crate) thing: T,
    out: W,
    clear: bool,
    append_newline: bool,
    inline_mode: bool,
    previous_inline_height: u16,
    used_height: u16, // Actual height to render (for inline mode, avoids buffer reallocation)
}

impl<W, T> Render<W, T>
where
    W: std::io::Write,
    T: Draw,
{
    pub fn render(&mut self) -> std::io::Result<()> {
        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();

        debug!(
            target: "render",
            "render start: mode={}, size={}x{}",
            if self.inline_mode { "inline" } else { "fullscreen" },
            self.buffer.size().width,
            self.buffer.size().height
        );

        // the position in chunk should be (0, 0), the render already move to the target position
        let buffer_size = self.buffer.size();
        // In inline mode, use used_height to ensure borders render within visible area
        let render_size = if self.inline_mode {
            Size {
                width: buffer_size.width,
                height: self.used_height,
            }
        } else {
            buffer_size
        };
        let chunk = Chunk::new(&mut self.buffer, render_size.into())?;
        self.thing.draw(chunk)?;

        if self.clear {
            queue_with_log!(self.out, Clear(ClearType::All))?;
        }

        // Inline mode: use relative positioning with line-level differential rendering
        if self.inline_mode {
            self.render_inline()?;
        } else {
            self.render_fullscreen()?;
        }

        if self.append_newline {
            queue_with_log!(self.out, Print("\n"))?;
        }

        // ensure the output is flushed to terminal
        self.out.flush()?;

        // Save current buffer for next frame's diff
        self.buffer.clear();

        #[cfg(debug_assertions)]
        debug!(
            target: "render",
            "render complete: duration={:.2?}",
            start.elapsed()
        );

        Ok(())
    }

    fn render_fullscreen(&mut self) -> std::io::Result<()> {
        let buffer_size = self.buffer.size();
        // Fullscreen mode: use absolute positioning with differential rendering
        if let Some(diff_iter) = self.buffer.diff() {
            // We have previous frame, do differential rendering
            debug!(
                target: "render::diff",
                "using differential rendering (fullscreen)"
            );
            let mut has_color_cache = false;
            let mut has_attr_cache = false;
            for (x, y, cell) in diff_iter {
                if y >= buffer_size.height {
                    break;
                }

                // Move cursor to this position (absolute positioning)
                queue_with_log!(self.out, MoveTo(self.pos.x + x, self.pos.y + y))?;

                // Reset color if we had color before but current cell doesn't
                if has_color_cache && !cell.has_color() {
                    queue_with_log!(self.out, ResetColor)?;
                }

                // Reset attributes if we had attributes before but current cell doesn't
                if has_attr_cache && !cell.has_attr() {
                    queue_with_log!(self.out, SetAttribute(Attribute::Reset))?;
                }

                cell.queue(&mut self.out)?;
                has_color_cache = cell.has_color();
                has_attr_cache = cell.has_attr();
            }
        } else {
            // First render or no previous buffer, do full render
            debug!(
                target: "render::diff",
                "using full rendering (fullscreen)"
            );
            // Optimize by rendering line by line instead of cell by cell
            let mut has_color_cache = false;
            let mut has_attr_cache = false;
            let mut current_line = 0u16;
            let mut need_move = true;

            for (_x, y, cell) in self.buffer.all_cells() {
                if y >= buffer_size.height {
                    break;
                }

                // Move to the start of a new line when y changes
                if y != current_line {
                    current_line = y;
                    need_move = true;
                }

                // Only move cursor when starting a new line
                if need_move {
                    queue_with_log!(self.out, MoveTo(self.pos.x, self.pos.y + y))?;
                    need_move = false;
                }

                // Reset color if we had color before but current cell doesn't
                if has_color_cache && !cell.has_color() {
                    queue_with_log!(self.out, ResetColor)?;
                }

                // Reset attributes if we had attributes before but current cell doesn't
                if has_attr_cache && !cell.has_attr() {
                    queue_with_log!(self.out, SetAttribute(Attribute::Reset))?;
                }

                cell.queue(&mut self.out)?;
                has_color_cache = cell.has_color();
                has_attr_cache = cell.has_attr();
            }
        }
        Ok(())
    }

    fn render_inline(&mut self) -> std::io::Result<()> {
        let current_height = self.used_height;

        debug!(
            target: "render::inline",
            "inline render: lines={}, prev_lines={}",
            current_height,
            self.previous_inline_height
        );

        // Move cursor back to the start position first
        // NOTE: After rendering, cursor is at the end of the last line (not next line's start)
        // because we skip MoveToNextLine for the last line to prevent scrolling.
        if self.buffer.previous().is_some() && self.previous_inline_height > 0 {
            // Move to line start first
            queue_with_log!(self.out, Print("\r"))?;
            // Then move up to first line (if not already there)
            if self.previous_inline_height > 1 {
                queue_with_log!(
                    self.out,
                    MoveToPreviousLine(self.previous_inline_height - 1)
                )?;
            }
        }

        // If height decreased, clear the extra lines from previous frame
        // Cursor is now at the start of line 0, so we need to:
        // 1. Move down to where the extra lines start (line current_height)
        // 2. Clear those lines
        if self.previous_inline_height > current_height {
            let extra_lines = self.previous_inline_height - current_height;
            // Move down to line current_height
            if current_height > 0 {
                queue_with_log!(self.out, MoveToNextLine(current_height))?;
            }
            // Clear extra lines (be careful not to move past the last line)
            for i in 0..extra_lines {
                queue_with_log!(self.out, Clear(ClearType::CurrentLine))?;
                // Don't move to next line after clearing the last extra line
                if i < extra_lines - 1 {
                    queue_with_log!(self.out, MoveToNextLine(1))?;
                }
            }
            // Move back to line 0
            let lines_to_move_back = current_height + extra_lines - 1;
            if lines_to_move_back > 0 {
                queue_with_log!(self.out, Print("\r"))?;
                queue_with_log!(self.out, MoveToPreviousLine(lines_to_move_back))?;
            }
        }

        // Render with line-level diffing
        let mut has_color_cache = false;
        let mut has_attr_cache = false;
        for line_diff in self.buffer.diff_lines() {
            if line_diff.line_num >= current_height {
                break;
            }

            let is_last_line = line_diff.line_num == current_height - 1;

            match line_diff.state {
                crate::buffer::LineState::Unchanged => {
                    // Line hasn't changed, just move to next line without re-rendering
                    // Skip MoveToNextLine for the last line to prevent terminal scrolling
                    if !is_last_line {
                        queue_with_log!(self.out, MoveToNextLine(1))?;
                    }
                }
                crate::buffer::LineState::Changed {
                    cells,
                    current_len,
                    previous_len,
                } => {
                    // Line has changed, render it with smart overwrite
                    for cell in cells {
                        // Reset color if we had color before but current cell doesn't
                        if has_color_cache && !cell.has_color() {
                            queue_with_log!(self.out, ResetColor)?;
                        }

                        // Reset attributes if we had attributes before but current cell doesn't
                        if has_attr_cache && !cell.has_attr() {
                            queue_with_log!(self.out, SetAttribute(Attribute::Reset))?;
                        }

                        cell.queue(&mut self.out)?;
                        has_color_cache = cell.has_color();
                        has_attr_cache = cell.has_attr();
                    }

                    // If current line is shorter than previous, clear trailing spaces
                    if current_len < previous_len {
                        let trailing_spaces = previous_len - current_len;
                        for _ in 0..trailing_spaces {
                            queue_with_log!(self.out, Print(' '))?;
                        }
                    }

                    // Move to next line, but skip for the last line to prevent terminal scrolling
                    // CRITICAL: When rendering at the bottom of the terminal, MoveToNextLine
                    // would cause the terminal to scroll up, breaking cursor position tracking
                    if !is_last_line {
                        // in inline mode, "\n" could make the terminal scroll up
                        // i know this is wired, but the "\n" doesn't move cursor
                        // println!("position: {:?}", cursor::position().unwrap()); // --> position: (0, x)
                        // queue_with_log!(stdout(), Print("\r\n")).unwrap();                // --> empty line
                        // println!("position: {:?}", cursor::position().unwrap()); // --> position: (0, x) <- this is not x+1!
                        queue_with_log!(self.out, Print("\r\n"))?;
                        queue_with_log!(self.out, MoveToPreviousLine(1))?;
                        queue_with_log!(self.out, MoveToNextLine(1))?;
                    }
                }
            }
        }

        // Update previous height for next frame
        self.previous_inline_height = current_height;
        Ok(())
    }

    pub(crate) fn from_builder(builder: &Builder, thing: T, writer: W) -> Self
    where
        T: Draw,
        W: std::io::Write,
    {
        Self {
            pos: builder.pos,
            buffer: Buffer::new(builder.size),
            thing,
            out: writer,
            clear: builder.clear,
            append_newline: builder.append_newline,
            inline_mode: builder.inline_mode,
            previous_inline_height: 0,
            used_height: builder.size.height, // Initialize to buffer capacity
        }
    }
}

// do not impl DrawUpdate for Render, it's not a drawable thing
impl<W, T> Render<W, T>
where
    W: std::io::Write,
    T: DrawUpdate + Send + Sync + 'static,
{
    pub fn update(&mut self) -> Result<bool, DrawErr> {
        self.thing.update()
    }

    pub fn on_events(&mut self, events: &[crossterm::event::Event]) -> Result<(), DrawErr> {
        self.thing.on_events(events)
    }

    /// Resize the render buffer
    pub fn resize(&mut self, new_size: Size) {
        self.buffer.resize(new_size);
        self.used_height = new_size.height; // Update used_height when resizing
    }

    /// Set the used height for rendering (inline mode optimization)
    ///
    /// This changes the actual height rendered without reallocating the buffer.
    /// Used in inline mode to avoid frequent buffer reallocations.
    /// The height must not exceed the buffer's capacity.
    pub fn set_used_height(&mut self, height: u16) {
        let max_height = self.buffer.size().height;
        self.used_height = height.min(max_height);
    }

    /// Get current render size
    pub fn size(&self) -> Size {
        self.buffer.size()
    }

    /// Get reference to the drawable thing
    pub fn thing(&self) -> &T {
        &self.thing
    }

    /// Get mutable reference to the drawable thing
    pub fn thing_mut(&mut self) -> &mut T {
        &mut self.thing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{area::Size, style::Stylized, Draw, DrawErr};

    struct Text {
        lines: Vec<String>,
    }

    impl Text {
        fn new(text: &str) -> Self {
            Self {
                lines: text.split("\n").map(|x| x.to_string()).collect(),
            }
        }
    }

    impl Draw for Text {
        fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
            let mut width = 0;
            let mut height = 0;
            for (y, line) in self.lines.iter().enumerate() {
                let mut x = 0;
                for c in line.chars() {
                    if let Ok(w) = chunk.set(x as u16, y as u16, Stylized::plain(c)) {
                        x += w;
                    } else {
                        break;
                    }
                }
                if x > width {
                    width = x;
                }
                height = y + 1;
            }
            Ok((width as u16, height as u16).into())
        }
    }

    #[test]
    fn test_render_simple() {
        let thing = Text::new("Hello, world!\nHello, world!");
        let mut buffer = Vec::new();
        let mut render = Builder::new()
            .size((10, 2))
            .clear(false)
            .build_render(thing, &mut buffer);
        render.render().unwrap();

        let result = String::from_utf8(buffer).unwrap();

        assert_eq!(result, "\u{1b}[1;1HHello, wor\u{1b}[2;1HHello, wor");
    }

    #[test]
    fn test_render_clear() {
        let thing = Text::new("Hello, world!");
        let mut buffer = Vec::new();
        let mut render = Builder::new()
            .size((20, 1))
            .clear(true)
            .build_render(thing, &mut buffer);
        render.render().unwrap();

        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(result, "\u{1b}[2J\u{1b}[1;1HHello, world!       ");
    }

    #[test]
    fn test_render_cut_off() {
        let thing = Text::new("你好啊啊啊");
        let mut buffer = Vec::new();
        let mut render = Builder::new()
            .size((5, 1))
            .clear(false)
            .build_render(thing, &mut buffer);
        render.render().unwrap();

        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(result, "\u{1b}[1;1H你好 ");
    }

    #[test]
    fn test_render_position() {
        let thing = Text::new("test line 1\ntest line 2");
        let mut buffer = Vec::new();
        let mut render = Builder::new()
            .size((10, 2))
            .clear(false)
            .pos((10, 10))
            .build_render(thing, &mut buffer);
        render.render().unwrap();

        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(result, "\u{1b}[11;11Htest line \u{1b}[12;11Htest line ");
    }
}
