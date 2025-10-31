//! Rendering buffer that bridges to the render package

use crate::style::{Color, Modifiers, Style};
use crate::widget::common::Rect;

/// Buffer for widget rendering
///
/// This is a simplified buffer that will bridge to the render package's
/// actual rendering implementation.
#[derive(Debug)]
pub struct Buffer {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
    previous: Option<Vec<Cell>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub symbol: char,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub modifiers: Modifiers,
}

impl Cell {
    fn empty() -> Self {
        Self {
            symbol: ' ',
            fg: None,
            bg: None,
            modifiers: Modifiers::empty(),
        }
    }
}

impl Buffer {
    /// Create a new buffer with the specified dimensions
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            cells: vec![Cell::empty(); size],
            previous: None,
        }
    }

    /// Get the buffer dimensions
    pub fn area(&self) -> Rect {
        Rect::new(0, 0, self.width, self.height)
    }

    /// Set a character at the specified position
    pub fn set_char(&mut self, x: u16, y: u16, ch: char) {
        if let Some(cell) = self.get_cell_mut(x, y) {
            cell.symbol = ch;
        }
    }

    /// Set a string starting at the specified position
    pub fn set_string(&mut self, x: u16, y: u16, string: &str, style: Style) {
        let mut current_x = x;
        for ch in string.chars() {
            if current_x >= self.width {
                break;
            }
            if let Some(cell) = self.get_cell_mut(current_x, y) {
                cell.symbol = ch;
                cell.fg = style.fg_color;
                cell.bg = style.bg_color;
                cell.modifiers = style.modifiers;
            }
            current_x += 1;
        }
    }

    /// Fill a rectangular area with a character
    pub fn fill(&mut self, area: Rect, ch: char) {
        for y in area.y..(area.y + area.height) {
            for x in area.x..(area.x + area.width) {
                self.set_char(x, y, ch);
            }
        }
    }

    /// Fill background color for an area
    pub fn fill_bg(&mut self, area: Rect, color: Color) {
        for y in area.y..(area.y + area.height) {
            for x in area.x..(area.x + area.width) {
                if let Some(cell) = self.get_cell_mut(x, y) {
                    cell.bg = Some(color);
                }
            }
        }
    }

    /// Draw a border around the area
    pub fn draw_border(&mut self, area: Rect, style: crate::style::BorderStyle) {
        let chars = style.chars();

        // Top and bottom borders
        for x in (area.x + 1)..(area.x + area.width - 1) {
            self.set_char(x, area.y, chars.horizontal);
            self.set_char(x, area.y + area.height - 1, chars.horizontal);
        }

        // Left and right borders
        for y in (area.y + 1)..(area.y + area.height - 1) {
            self.set_char(area.x, y, chars.vertical);
            self.set_char(area.x + area.width - 1, y, chars.vertical);
        }

        // Corners
        self.set_char(area.x, area.y, chars.top_left);
        self.set_char(area.x + area.width - 1, area.y, chars.top_right);
        self.set_char(area.x, area.y + area.height - 1, chars.bottom_left);
        self.set_char(
            area.x + area.width - 1,
            area.y + area.height - 1,
            chars.bottom_right,
        );
    }

    /// Get text at a specific position (for testing)
    pub fn get_text_at(&self, x: u16, y: u16) -> String {
        if let Some(cell) = self.get_cell(x, y) {
            cell.symbol.to_string()
        } else {
            String::new()
        }
    }

    /// Get a cell at the specified position
    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        if x < self.width && y < self.height {
            let index = (y as usize) * (self.width as usize) + (x as usize);
            self.cells.get(index)
        } else {
            None
        }
    }

    fn get_cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.get(x, y)
    }

    fn get_cell_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        if x < self.width && y < self.height {
            let index = (y as usize) * (self.width as usize) + (x as usize);
            self.cells.get_mut(index)
        } else {
            None
        }
    }

    /// Render the buffer contents to terminal
    pub fn flush(&mut self) -> crate::error::Result<()> {
        use crossterm::{
            cursor::MoveTo,
            queue,
            style::{Attribute, ResetColor, SetAttribute},
            terminal::Clear,
        };
        use std::io::{stdout, Write};

        let mut stdout = stdout();

        // Check if we need a full redraw (first render or terminal resize)
        let needs_full_redraw = self.previous.is_none()
            || self
                .previous
                .as_ref()
                .map_or(false, |prev| prev.len() != self.cells.len());

        if needs_full_redraw {
            // Clear screen only on first render or resize
            queue!(stdout, Clear(crossterm::terminal::ClearType::All))?;

            // Track current style state to minimize escape codes
            let mut prev_fg: Option<Color> = None;
            let mut prev_bg: Option<Color> = None;
            let mut prev_mods = Modifiers::empty();

            for y in 0..self.height {
                queue!(stdout, MoveTo(0, y))?;
                for x in 0..self.width {
                    if let Some(cell) = self.get_cell(x, y) {
                        self.apply_style(
                            &mut stdout,
                            cell,
                            &mut prev_fg,
                            &mut prev_bg,
                            &mut prev_mods,
                        )?;
                        write!(stdout, "{}", cell.symbol)?;
                    }
                }
            }
        } else {
            // Differential rendering - only update changed cells
            let prev_cells = self.previous.as_ref().unwrap();

            let mut prev_fg: Option<Color> = None;
            let mut prev_bg: Option<Color> = None;
            let mut prev_mods = Modifiers::empty();
            let mut last_pos: Option<(u16, u16)> = None;

            for y in 0..self.height {
                for x in 0..self.width {
                    let index = (y as usize) * (self.width as usize) + (x as usize);
                    if let (Some(new_cell), Some(old_cell)) =
                        (self.cells.get(index), prev_cells.get(index))
                    {
                        // Only render if the cell has changed
                        if new_cell != old_cell {
                            // Move cursor if needed
                            if last_pos != Some((x, y)) {
                                queue!(stdout, MoveTo(x, y))?;
                            }

                            self.apply_style(
                                &mut stdout,
                                new_cell,
                                &mut prev_fg,
                                &mut prev_bg,
                                &mut prev_mods,
                            )?;
                            write!(stdout, "{}", new_cell.symbol)?;
                            last_pos = Some((x + 1, y));
                        }
                    }
                }
            }
        }

        // Reset all attributes at the end
        queue!(stdout, SetAttribute(Attribute::Reset), ResetColor)?;
        stdout.flush()?;

        // Store current buffer as previous for next frame
        self.previous = Some(self.cells.clone());

        Ok(())
    }

    /// Apply style changes to the output stream
    fn apply_style<W: std::io::Write>(
        &self,
        stdout: &mut W,
        cell: &Cell,
        prev_fg: &mut Option<Color>,
        prev_bg: &mut Option<Color>,
        prev_mods: &mut Modifiers,
    ) -> crate::error::Result<()> {
        use crossterm::{
            queue,
            style::{Attribute, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
        };

        // Only change colors if they're different from previous
        if cell.fg != *prev_fg {
            if let Some(fg) = cell.fg {
                queue!(stdout, SetForegroundColor(color_to_crossterm(fg)))?;
            } else if prev_fg.is_some() {
                queue!(stdout, ResetColor)?;
            }
            *prev_fg = cell.fg;
        }

        if cell.bg != *prev_bg {
            if let Some(bg) = cell.bg {
                queue!(stdout, SetBackgroundColor(color_to_crossterm(bg)))?;
            } else if prev_bg.is_some() {
                queue!(stdout, ResetColor)?;
            }
            *prev_bg = cell.bg;
        }

        // Only update modifiers if changed
        if cell.modifiers != *prev_mods {
            // Reset all if going from modified to plain
            if !cell.modifiers.is_empty() || !prev_mods.is_empty() {
                queue!(stdout, SetAttribute(Attribute::Reset))?;
            }

            // Apply new modifiers
            if cell.modifiers.contains_bold() {
                queue!(stdout, SetAttribute(Attribute::Bold))?;
            }
            if cell.modifiers.contains_italic() {
                queue!(stdout, SetAttribute(Attribute::Italic))?;
            }
            if cell.modifiers.contains_underlined() {
                queue!(stdout, SetAttribute(Attribute::Underlined))?;
            }

            *prev_mods = cell.modifiers;
        }

        Ok(())
    }
}

fn color_to_crossterm(color: Color) -> crossterm::style::Color {
    use crossterm::style::Color as CC;
    match color {
        Color::Black => CC::Black,
        Color::Red => CC::DarkRed,
        Color::Green => CC::DarkGreen,
        Color::Yellow => CC::DarkYellow,
        Color::Blue => CC::DarkBlue,
        Color::Magenta => CC::DarkMagenta,
        Color::Cyan => CC::DarkCyan,
        Color::White => CC::White,
        Color::Rgb(r, g, b) => CC::Rgb { r, g, b },
        Color::Indexed(i) => CC::AnsiValue(i),
    }
}
