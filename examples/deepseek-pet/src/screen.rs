//! The shell's visible text, independent of the Sixel overlay.
use std::{io::Write, num::NonZeroU16};
use vt100::{Cell, Color, Parser};

pub struct Screen(Parser);

impl Screen {
    pub fn new(rows: u16, cols: u16, (col, row): (u16, u16)) -> Option<Self> {
        let (rows, cols) = dimensions(rows, cols)?;
        let mut parser = Parser::new(rows, cols, 0);
        parser.process(format!("\x1b[{row};{col}H").as_bytes());
        Some(Self(parser))
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> bool {
        let Some((rows, cols)) = dimensions(rows, cols) else {
            return false;
        };
        self.0.set_size(rows, cols);
        true
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.0.process(bytes);
    }

    // Apply a deliberate bite to the backing text too. Do not disturb the
    // child's DECSC save slot by injecting another save/restore pair here.
    pub fn consume(&mut self, erase: &[u8]) {
        let screen = self.0.screen();
        let cursor = screen.cursor_state_formatted();
        let attrs = screen.attributes_formatted();
        self.feed(erase);
        self.feed(cursor.as_bytes());
        self.feed(attrs.as_bytes());
    }

    fn restore(&self, col: u16, row: u16, width: u16, out: &mut Vec<u8>) {
        let screen = self.0.screen();
        let y = row - 1;
        let mut x = col - 1;
        let end = x.saturating_add(width);
        if screen.cell(y, x).is_some_and(Cell::is_wide_continuation) {
            x = x.saturating_sub(1);
        }
        let mut pen = None;
        let mut next_col = None;
        while x < end {
            let Some(cell) = screen.cell(y, x) else { break };
            let after = x.saturating_add(if cell.is_wide() { 2 } else { 1 });
            let style = Style::from(cell);
            if cell.has_contents() || style != Style::default() {
                if next_col != Some(x) {
                    write!(out, "\x1b[{row};{}H", x + 1).unwrap();
                }
                if pen != Some(style) {
                    style.write(out);
                    pen = Some(style);
                }
                let text = if cell.has_contents() {
                    cell.contents()
                } else {
                    " "
                };
                out.extend_from_slice(text.as_bytes());
                next_col = Some(after);
            }
            x = after;
        }
        if pen.is_some_and(|style| style != Style::default()) {
            out.extend_from_slice(b"\x1b[0m");
        }
    }
}

// vt100 uses 32 bytes per cell. Bound each grid to 8 MiB even if the
// PTY advertises unrealistic dimensions. No scrollback is retained.
fn dimensions(rows: u16, cols: u16) -> Option<(NonZeroU16, NonZeroU16)> {
    if usize::from(rows) * usize::from(cols) > 256 * 1024 {
        return None;
    }
    Some((NonZeroU16::new(rows)?, NonZeroU16::new(cols)?))
}

pub fn clear_span(screen: Option<&Screen>, col: u16, row: u16, width: u16, out: &mut Vec<u8>) {
    write!(out, "\x1b[{row};{col}H\x1b[{width}X").unwrap();
    if let Some(screen) = screen {
        screen.restore(col, row, width, out);
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
struct Style {
    fg: Color,
    bg: Color,
    modes: [bool; 5],
}

impl From<&Cell> for Style {
    fn from(c: &Cell) -> Self {
        Self {
            fg: c.fgcolor(),
            bg: c.bgcolor(),
            modes: [c.bold(), c.dim(), c.italic(), c.underline(), c.inverse()],
        }
    }
}

impl Style {
    fn write(self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"\x1b[0");
        for (enabled, code) in self.modes.into_iter().zip([1, 2, 3, 4, 7]) {
            if enabled {
                write!(out, ";{code}").unwrap();
            }
        }
        for (prefix, color) in [(38, self.fg), (48, self.bg)] {
            match color {
                Color::Default => (),
                Color::Idx(i) => write!(out, ";{prefix};5;{i}").unwrap(),
                Color::Rgb(r, g, b) => write!(out, ";{prefix};2;{r};{g};{b}").unwrap(),
            }
        }
        out.push(b'm');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redraws_only_the_clipped_span_including_wide_glyphs_and_styled_blanks() {
        let mut screen = Screen::new(6, 30, (4, 3)).unwrap();
        screen.feed("鲸e\u{301}\x1b[1;3;4;7;38;2;12;34;56;48;5;23mab".as_bytes());
        screen.feed(b"\x1b[2X\x1b[0m\x1b[3;11H!");
        let mut dest = Screen::new(6, 30, (1, 1)).unwrap();
        dest.feed(b"\x1b[3;1H+++XXXXXXX!");
        let mut out = Vec::new();
        clear_span(Some(&screen), 5, 3, 6, &mut out);
        dest.feed(&out);
        let at = |s: &Screen, x| s.0.screen().cell(2, x).unwrap().clone();
        for x in 3..8 {
            assert_eq!(at(&dest, x), at(&screen, x));
        }
        for x in 8..10 {
            assert_eq!(Style::from(&at(&dest, x)), Style::from(&at(&screen, x)));
        }
        assert_eq!(at(&dest, 2).contents(), "+");
        assert_eq!(at(&dest, 10).contents(), "!");
    }

    #[test]
    fn shell_scroll_alternate_resize_and_meals_preserve_the_current_text() {
        let mut screen = Screen::new(4, 20, (1, 3)).unwrap();
        screen.feed(b"food\r\nnext\r\n");
        screen.feed(b"\x1b[?1049h\x1b[Halt\x1b[?1049l");
        screen.resize(4, 18);
        screen.feed(b"\x1b[32m");
        let before = screen.0.screen().cursor_position();
        screen.consume(b"\x1b[49m\x1b[2;2H\x1b[X");
        assert_eq!(screen.0.screen().cursor_position(), before);
        assert_eq!(screen.0.screen().fgcolor(), Color::Idx(2));
        assert_eq!(screen.0.screen().rows(0, 18).nth(1).unwrap(), "f od");
        assert!(!screen.resize(u16::MAX, u16::MAX));
    }
}
