use crate::dialogue::{CommandEvent, Dialogue};
use rsille::render::{
    area::{Area, Size},
    chunk::Chunk,
    text::TextMetrics,
    Builder, Draw, DrawErr,
};
use rsille::tui::{
    layout::border_renderer::{border_chars, render_border},
    style::{BorderStyle, Color, Style},
};
use std::{
    io,
    time::{Duration, Instant},
};

const BUBBLE_READ_TIME: Duration = Duration::from_millis(1500);
const BUBBLE_LIFETIME: Duration = Duration::from_secs(3);

#[derive(Default)]
pub(super) struct Speech<'a> {
    dialogue: Dialogue,
    pub(super) latest_speech: Option<(&'a str, Instant)>,
    pub(super) held_speech: Option<(&'a str, Instant)>,
}

impl<'a> Speech<'a> {
    pub(super) fn clear_preview(&mut self) {
        self.latest_speech = None;
    }

    pub(super) fn notice_command(&mut self, event: CommandEvent<'a>) {
        let text = self.dialogue.choose(event);
        self.latest_speech = Some((text, Instant::now() + BUBBLE_LIFETIME));
    }

    pub(super) fn place(
        &mut self,
        col: u16,
        row: u16,
        size: (u16, u16),
        cols: u16,
        rows: u16,
    ) -> Option<Bubble<'a>> {
        let now = Instant::now();
        self.latest_speech = self.latest_speech.filter(|&(_, until)| now < until);
        let bubble = self
            .held_speech
            .filter(|&(_, until)| now < until)
            .or(self.latest_speech)
            .and_then(|(text, _)| Bubble::place(text, col, row, size, cols, rows));
        // Shell output can erase and repaint the same bubble on every keystroke.
        // Start its reading time only when a different line first fits on screen.
        let visible = bubble.map(|b| b.text);
        if visible != self.held_speech.map(|(text, _)| text) {
            self.held_speech = visible.map(|text| (text, now + BUBBLE_READ_TIME));
            if let Some(text) = visible {
                self.latest_speech = Some((text, now + BUBBLE_LIFETIME));
            }
        }
        bubble
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Bubble<'a> {
    pub(super) text: &'a str,
    pub(super) area: Area,
    above: bool,
    tail: u16,
}

impl<'a> Bubble<'a> {
    fn place(
        text: &'a str,
        col: u16,
        row: u16,
        size: (u16, u16),
        cols: u16,
        rows: u16,
    ) -> Option<Self> {
        let width = u16::try_from(TextMetrics::default().width(text))
            .ok()?
            .checked_add(4)?;
        if width >= cols {
            return None;
        }
        let room_above = row - 1;
        let room_below = rows - (room_above + size.1);
        let height = if room_above.max(room_below) >= 3 {
            3
        } else {
            1
        };
        let above = room_above >= height;
        let row = if above {
            row - height
        } else if room_below >= height {
            row + size.1
        } else {
            return None;
        };
        let center = col + size.0 / 2;
        let col = center.saturating_sub(width / 2).clamp(1, cols - width);
        Some(Self {
            text,
            area: Area::new((col - 1, row - 1).into(), (width, height).into()),
            above,
            tail: (center - col).clamp(1, width - 2),
        })
    }

    pub(super) fn erase(
        self,
        cols: u16,
        rows: u16,
        screen: Option<&crate::screen::Screen>,
        out: &mut Vec<u8>,
    ) {
        let viewport = Area::from(Size::from((cols, rows)));
        let Some(visible) = self.area.clamp_to(&viewport) else {
            return;
        };
        let col = visible.x() + 1;
        let width = visible.width();
        out.extend_from_slice(b"\x1b[49m");
        for y in visible.y()..visible.y() + visible.height() {
            crate::screen::clear_span(screen, col, y + 1, width, out);
        }
    }

    pub(super) fn render(self, out: &mut Vec<u8>) -> io::Result<()> {
        Builder::new()
            .size(self.area.size().into())
            .pos(self.area.pos().into())
            .clear(false)
            .build_render(self, out)
            .render()
    }
}

impl Draw for Bubble<'_> {
    fn draw(&mut self, mut chunk: Chunk<'_>) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        let border = BorderStyle::Rounded;
        let outline = Style::default().fg(Color::Indexed(116)).to_render_style();
        let lettering = Style::default().fg(Color::Indexed(195)).to_render_style();
        if size.height == 1 {
            let side = border_chars(&chunk, border).vertical;
            chunk.set_char(0, 0, side, outline)?;
            chunk.set_char(size.width - 1, 0, side, outline)?;
        } else {
            render_border(&mut chunk, border, outline);
            let (row, tip) = if self.above {
                (size.height - 1, '▾')
            } else {
                (0, '▴')
            };
            chunk.set_char(self.tail, row, tip, outline)?;
        }
        chunk.set_string(2, size.height / 2, self.text, lettering)?;
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bubble_points_at_the_whale_and_clears_only_visible_rows() {
        for (row, top, tail) in [(1, 3, '▴'), (7, 4, '▾')] {
            let bubble = Bubble::place("你好", 17, row, (3, 2), 20, 8).unwrap();
            assert_eq!(bubble.area, Area::new((11, top - 1).into(), (8, 3).into()));
            let mut out = Vec::new();
            bubble.render(&mut out).unwrap();
            let drawing = String::from_utf8(out).unwrap();
            assert!(drawing.contains(tail));
            assert_eq!(drawing.matches('─').count(), 11); // two six-cell edges, one pointer
            let mut erase = Vec::new();
            bubble.erase(15, top + 1, None, &mut erase);
            assert_eq!(
                erase,
                format!("\x1b[49m\x1b[{top};12H\x1b[4X\x1b[{};12H\x1b[4X", top + 1).as_bytes()
            );
        }
        assert_eq!(
            Bubble::place("你好", 1, 2, (3, 2), 20, 4)
                .unwrap()
                .area
                .height(),
            1
        );
    }

    #[test]
    fn bubble_uses_framework_text_metrics_and_renders_only_its_rectangle() {
        // A CJK glyph, an ASCII letter with a combining accent, and an emoji cluster.
        let message = "鲸e\u{301}👩‍💻";
        let bubble = Bubble::place(message, 5, 7, (3, 2), 20, 10).unwrap();
        assert_eq!(bubble.area.width(), 9);
        let mut bytes = Vec::new();
        bubble.render(&mut bytes).unwrap();
        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains(message));
        assert!(!output.contains("\x1b[2J") && !output.contains('\n'));
    }
}
