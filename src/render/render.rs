//! Buffered terminal output with cell-level and line-level frame comparison.

use crossterm::{
    cursor::{MoveTo, MoveToNextLine, MoveToPreviousLine, RestorePosition, SavePosition},
    style::{Attribute, Attributes, Color, Colors, Print, ResetColor, SetAttributes, SetColors},
    terminal::{BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate},
};
use log::debug;

use crate::render::{
    area::{Position, Size},
    buffer::Buffer,
    chunk::Chunk,
    style::Stylized,
    Builder, Draw, DrawErr, DrawUpdate,
};

/// Tracks the style actually written to the terminal within one frame.
/// Missing cell colors mean terminal defaults, not inheritance from the
/// previously emitted cell (which may be far away during a sparse diff).
#[derive(Default)]
struct TerminalStyleState {
    foreground: Option<Color>,
    background: Option<Color>,
    attributes: Attributes,
}

impl TerminalStyleState {
    fn reset(&mut self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        crossterm::queue!(out, ResetColor)?;
        *self = Self::default();
        Ok(())
    }

    fn queue(&mut self, cell: &Stylized, out: &mut impl std::io::Write) -> std::io::Result<()> {
        if cell.is_empty() || cell.width() == 0 {
            return Ok(());
        }

        let attributes = cell.style.attr.unwrap_or_default();
        if attributes != self.attributes || attributes.has(Attribute::Reset) {
            // SGR 0 also clears colors. Apply attributes first and reapply
            // colors afterwards, including when the cell explicitly resets.
            if !self.attributes.is_empty() || attributes.has(Attribute::Reset) {
                self.reset(out)?;
            }
            crossterm::queue!(out, SetAttributes(attributes))?;
            self.attributes = attributes;
        }

        let foreground = cell
            .style
            .colors
            .and_then(|c| c.foreground)
            .filter(|c| *c != Color::Reset);
        let background = cell
            .style
            .colors
            .and_then(|c| c.background)
            .filter(|c| *c != Color::Reset);
        let changes = Colors {
            foreground: (foreground != self.foreground)
                .then_some(foreground.unwrap_or(Color::Reset)),
            background: (background != self.background)
                .then_some(background.unwrap_or(Color::Reset)),
        };
        crossterm::queue!(out, SetColors(changes))?;
        self.foreground = foreground;
        self.background = background;
        cell.queue_symbol(out)
    }
}

/// Draws `T` into a reusable buffer and writes terminal commands to `W`.
///
/// Construct with [`Builder::build_render`]. Absolute rendering emits changed
/// cells; inline rendering compares whole lines and uses relative cursor
/// movement. This type does not set up or restore terminal modes.
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
    raster: super::raster::RasterPresenter,
}

impl<W, T> Render<W, T>
where
    W: std::io::Write,
    T: Draw,
{
    /// Draws a complete frame, writes its changes, and flushes the writer.
    ///
    /// Resets terminal colors and attributes before any drawing or clearing.
    /// Each emitted cell uses its resolved buffer style; unspecified colors
    /// and attributes use terminal defaults rather than the preceding cell.
    ///
    /// On success, saves the frame for comparison and clears the working
    /// buffer. The size returned by [`Draw::draw`] is ignored.
    ///
    /// # Errors
    ///
    /// Returns drawing failures converted to [`std::io::Error`], or errors
    /// from writing or flushing output. A failed frame is not saved.
    pub fn render(&mut self) -> std::io::Result<()> {
        let result = self.render_frame();
        if result.is_err() {
            // A failed draw is not a valid previous frame. Repaint from scratch
            // on retry; the presenter separately remembers uncertain graphics.
            self.buffer.clear_content();
            if self.raster.needs_protocol_recovery() {
                // Cancel partial DCS/CSI before releasing a synchronized frame.
                // Keep the presenter uncertain: a failed writer may reject this
                // cleanup too, so the next frame must repeat it and fully repaint.
                let _ = self.out.write_all(b"\x18\x1b\\");
                let _ = crossterm::queue!(self.out, EndSynchronizedUpdate, ResetColor);
                let _ = self.out.flush();
            }
        }
        result
    }

    fn render_frame(&mut self) -> std::io::Result<()> {
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

        // An appended LF can scroll even when Sixel itself cannot. Treat this
        // contract like inline output rather than commit stale screen positions.
        let raster_plan = self.raster.prepare(
            &self.buffer,
            self.pos,
            self.inline_mode || self.append_newline,
        )?;
        if self.raster.needs_protocol_recovery() {
            // A prior writer failure may have prevented even the best-effort
            // terminator from reaching the terminal. Cancel an incomplete DCS
            // before issuing SGR/ED commands on the recovered writer.
            self.out.write_all(b"\x18\x1b\\")?;
            crossterm::queue!(self.out, EndSynchronizedUpdate)?;
        }
        self.raster.begin(&raster_plan);
        // A flush is not an atomic terminal repaint. Hide the intermediate
        // clear/text/tile passes on terminals supporting DEC mode 2026, including
        // the last native frame's cleanup when switching to character output.
        let synchronized = raster_plan.native || raster_plan.clear;
        if synchronized {
            crossterm::queue!(self.out, BeginSynchronizedUpdate)?;
        }

        // Match the frame-local style cache to the real terminal before
        // drawing or erasing. Cursor movement and clearing do not reset SGR.
        crossterm::queue!(self.out, ResetColor)?;

        if raster_plan.clear || (self.clear && self.buffer.previous().is_none()) {
            // Native output is enabled only for hosts that verified ED 2 erases
            // graphics. Spaces/EL are not assumed to remove old Sixel pixels.
            crossterm::queue!(self.out, Clear(ClearType::All))?;
        }
        if raster_plan.clear {
            self.buffer.reset_diff();
        }

        // Inline mode: use relative positioning with line-level differential rendering
        if self.inline_mode {
            self.render_inline()?;
        } else if raster_plan.native {
            self.render_raster_characters()?;
        } else {
            self.render_fullscreen()?;
        }
        if !raster_plan.encoded.is_empty() {
            crossterm::queue!(self.out, SavePosition)?;
            for (x, y, bytes) in &raster_plan.encoded {
                crossterm::queue!(self.out, MoveTo(self.pos.x + x, self.pos.y + y))?;
                if let Err(error) = self.out.write_all(bytes) {
                    // Best effort DCS termination and cursor restoration on a
                    // recoverable writer failure. The next frame fully clears.
                    let _ = self.out.write_all(b"\x1b\\");
                    let _ = crossterm::queue!(self.out, RestorePosition, ResetColor);
                    let _ = self.out.flush();
                    return Err(error);
                }
            }
            crossterm::queue!(self.out, RestorePosition, ResetColor)?;
        }

        if self.append_newline {
            crossterm::queue!(self.out, Print("\n"))?;
        }

        if synchronized {
            crossterm::queue!(self.out, EndSynchronizedUpdate)?;
        }
        // Commit display history only after the entire synchronized frame flushes.
        self.out.flush()?;
        self.raster.commit(raster_plan);

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

    pub(crate) fn force_inline_viewport_follow(&mut self) -> std::io::Result<()> {
        if !self.inline_mode || self.used_height == 0 {
            return Ok(());
        }

        crossterm::queue!(self.out, Print("\r\n"), MoveToPreviousLine(1))?;
        self.out.flush()
    }

    // Ownership was resolved during drawing/blitting, so these cells cannot
    // overlap any native tile. This is not a fixed text-before-images z-order.
    fn render_raster_characters(&mut self) -> std::io::Result<()> {
        let mut style = TerminalStyleState::default();
        if let Some(diff) = self.buffer.diff() {
            for (x, y, cell) in diff {
                if self.buffer.raster_at((x, y).into()).is_none() {
                    crossterm::queue!(self.out, MoveTo(self.pos.x + x, self.pos.y + y))?;
                    style.queue(&cell.content, &mut self.out)?;
                }
            }
        } else {
            for (x, y, cell) in self.buffer.all_cells() {
                if self.buffer.raster_at((x, y).into()).is_none() {
                    crossterm::queue!(self.out, MoveTo(self.pos.x + x, self.pos.y + y))?;
                    style.queue(&cell.content, &mut self.out)?;
                }
            }
        }
        Ok(())
    }

    /// Changes presentation policy or explicit pixel metrics for subsequent frames.
    /// Old graphics are cleared on the next successful frame before text fallback.
    pub fn set_raster_output(&mut self, config: super::raster_output::RasterOutput) {
        self.raster.config = config;
    }

    /// Returns Sixel work counters and bounded cache occupancy.
    pub fn raster_stats(&self) -> super::raster_output::RasterStats {
        self.raster.stats()
    }

    pub(crate) fn set_alternate_screen(&mut self, enabled: bool) {
        self.raster.alternate_screen = enabled;
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
            let mut style = TerminalStyleState::default();
            for (x, y, cell) in diff_iter {
                if y >= buffer_size.height {
                    break;
                }

                // Move cursor to this position (absolute positioning)
                crossterm::queue!(self.out, MoveTo(self.pos.x + x, self.pos.y + y))?;

                style.queue(&cell.content, &mut self.out)?;
            }
        } else {
            // First render or no previous buffer, do full render
            debug!(
                target: "render::diff",
                "using full rendering (fullscreen)"
            );
            // Optimize by rendering line by line instead of cell by cell
            let mut style = TerminalStyleState::default();
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
                    crossterm::queue!(self.out, MoveTo(self.pos.x, self.pos.y + y))?;
                    need_move = false;
                }

                style.queue(&cell.content, &mut self.out)?;
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
            crossterm::queue!(self.out, Print("\r"))?;
            // Then move up to first line (if not already there)
            if self.previous_inline_height > 1 {
                crossterm::queue!(
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
                crossterm::queue!(self.out, MoveToNextLine(current_height))?;
            }
            // Clear extra lines (be careful not to move past the last line)
            for i in 0..extra_lines {
                crossterm::queue!(self.out, Clear(ClearType::CurrentLine))?;
                // Don't move to next line after clearing the last extra line
                if i < extra_lines - 1 {
                    crossterm::queue!(self.out, MoveToNextLine(1))?;
                }
            }
            // Move back to line 0
            let lines_to_move_back = current_height + extra_lines - 1;
            if lines_to_move_back > 0 {
                crossterm::queue!(self.out, Print("\r"))?;
                crossterm::queue!(self.out, MoveToPreviousLine(lines_to_move_back))?;
            }
        }

        // Render with line-level diffing
        let mut style = TerminalStyleState::default();
        for line_diff in self.buffer.diff_lines() {
            if line_diff.line_num >= current_height {
                break;
            }

            let is_last_line = line_diff.line_num == current_height - 1;

            match line_diff.state {
                crate::render::buffer::LineState::Unchanged => {
                    // Line hasn't changed, just move to next line without re-rendering
                    // Skip MoveToNextLine for the last line to prevent terminal scrolling
                    if !is_last_line {
                        crossterm::queue!(self.out, MoveToNextLine(1))?;
                    }
                }
                crate::render::buffer::LineState::Changed {
                    cells,
                    current_len,
                    previous_len,
                } => {
                    // Line has changed, render it with smart overwrite
                    for cell in cells {
                        style.queue(&cell.content, &mut self.out)?;
                    }

                    // If current line is shorter than previous, clear trailing spaces
                    if current_len < previous_len {
                        style.reset(&mut self.out)?;
                        let trailing_spaces = previous_len - current_len;
                        for _ in 0..trailing_spaces {
                            crossterm::queue!(self.out, Print(' '))?;
                        }
                    }

                    // Move to next line, but skip for the last line to prevent terminal scrolling
                    // CRITICAL: When rendering at the bottom of the terminal, MoveToNextLine
                    // would cause the terminal to scroll up, breaking cursor position tracking
                    if !is_last_line {
                        // in inline mode, "\n" could make the terminal scroll up
                        // i know this is wired, but the "\n" doesn't move cursor
                        // println!("position: {:?}", cursor::position().unwrap()); // --> position: (0, x)
                        // crossterm::queue!(stdout(), Print("\r\n")).unwrap();                // --> empty line
                        // println!("position: {:?}", cursor::position().unwrap()); // --> position: (0, x) <- this is not x+1!
                        crossterm::queue!(self.out, Print("\r\n"))?;
                        crossterm::queue!(self.out, MoveToPreviousLine(1))?;
                        crossterm::queue!(self.out, MoveToNextLine(1))?;
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
            raster: super::raster::RasterPresenter::new(
                builder.raster_output,
                builder.enable_alt_screen,
            ),
        }
    }
}

// do not impl DrawUpdate for Render, it's not a drawable thing
impl<W, T> Render<W, T>
where
    W: std::io::Write,
    T: DrawUpdate,
{
    /// Advances the application, returning whether it requests a redraw.
    ///
    /// Forwards errors from [`crate::render::Update::update`].
    pub fn update(&mut self) -> Result<bool, DrawErr> {
        self.thing.update()
    }

    /// Forwards events unchanged to the application and propagates its errors.
    ///
    /// This method does not translate inline mouse coordinates.
    pub fn on_events(&mut self, events: &[crossterm::event::Event]) -> Result<(), DrawErr> {
        self.thing.on_events(events)
    }

    /// Resizes the buffer and resets the used height to the new height.
    ///
    /// If dimensions change, current and previous frame contents are discarded.
    pub fn resize(&mut self, new_size: Size) {
        self.buffer.resize(new_size);
        self.used_height = new_size.height; // Update used_height when resizing
    }

    /// Set the used height for rendering (inline mode optimization)
    ///
    /// This changes the actual height rendered without reallocating the buffer.
    /// Used in inline mode to avoid frequent buffer reallocations.
    /// Heights exceeding the buffer's capacity are clamped to that capacity.
    pub fn set_used_height(&mut self, height: u16) {
        let max_height = self.buffer.size().height;
        self.used_height = height.min(max_height);
    }

    /// Get current render size
    pub fn size(&self) -> Size {
        self.buffer.size()
    }

    pub(crate) fn inline_origin(&self) -> Position {
        self.pos
    }

    pub(crate) fn inline_width(&self) -> u16 {
        self.buffer.size().width
    }

    pub(crate) fn inline_used_height(&self) -> u16 {
        self.used_height
    }

    pub(crate) fn set_inline_origin_y(&mut self, y: u16) {
        self.pos.y = y;
    }

    /// Get reference to the drawable thing
    pub fn thing(&self) -> &T {
        &self.thing
    }

    /// Get mutable reference to the drawable thing
    pub fn thing_mut(&mut self) -> &mut T {
        &mut self.thing
    }

    /// Check if the render buffer has pending changes that need to be rendered
    pub fn has_pending_changes(&self) -> bool {
        self.buffer.has_pending_changes()
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod style_tests;

#[cfg(test)]
#[path = "raster/render_tests.rs"]
mod raster_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{area::Size, style::Stylized, Draw, DrawErr};

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

        assert_eq!(
            result,
            "\u{1b}[0m\u{1b}[1;1HHello, wor\u{1b}[2;1HHello, wor"
        );
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
        assert_eq!(result, "\u{1b}[0m\u{1b}[2J\u{1b}[1;1HHello, world!       ");
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
        assert_eq!(result, "\u{1b}[0m\u{1b}[1;1H你好 ");
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
        assert_eq!(
            result,
            "\u{1b}[0m\u{1b}[11;11Htest line \u{1b}[12;11Htest line "
        );
    }
}
