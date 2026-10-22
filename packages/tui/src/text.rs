//! Text primitives and the shared editing core used by both input widgets.

use crate::event::{Event, KeyCode, KeyModifiers};
pub use render::text::*;

/// UTF-8 editing positions. Both positions always fall on grapheme boundaries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditBuffer {
    pub value: String,
    pub cursor: usize,
    pub anchor: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EditResult {
    pub handled: bool,
    pub changed: bool,
    pub submit: bool,
}

impl EditBuffer {
    pub fn new(value: String, cursor: usize, anchor: Option<usize>) -> Self {
        let mut editor = Self {
            value,
            cursor,
            anchor,
        };
        editor.repair();
        editor
    }

    pub fn repair(&mut self) {
        self.cursor = floor_boundary(&self.value, self.cursor);
        self.anchor = self.anchor.map(|a| floor_boundary(&self.value, a));
    }

    pub fn selection(&self) -> Option<std::ops::Range<usize>> {
        self.anchor
            .filter(|a| *a != self.cursor)
            .map(|a| a.min(self.cursor)..a.max(self.cursor))
    }

    fn insert(&mut self, text: &str) {
        let range = self.selection().unwrap_or(self.cursor..self.cursor);
        let end = range.start + text.len();
        self.value.replace_range(range, text);
        // Inserting a combining mark/ZWJ may join the following cluster too.
        self.cursor = ceil_boundary(&self.value, end);
        self.anchor = None;
    }

    fn delete(&mut self, backwards: bool) {
        let range = self.selection().unwrap_or_else(|| {
            if backwards {
                previous_boundary(&self.value, self.cursor)..self.cursor
            } else {
                self.cursor..next_boundary(&self.value, self.cursor)
            }
        });
        let start = range.start;
        self.value.replace_range(range, "");
        self.cursor = floor_boundary(&self.value, start);
        self.anchor = None;
    }

    pub fn handle(&mut self, event: &Event, metrics: TextMetrics, multiline: bool) -> EditResult {
        self.repair();
        let original = self.value.clone();
        let mut result = EditResult {
            handled: true,
            ..EditResult::default()
        };
        match event {
            Event::Paste(text) => {
                let text = text.replace("\r\n", "\n").replace('\r', "\n");
                let text: String = text
                    .chars()
                    .filter_map(|c| match c {
                        '\n' if !multiline => Some(' '),
                        '\n' | '\t' => Some(c),
                        c if c.is_control() => None,
                        c => Some(c),
                    })
                    .collect();
                if !text.is_empty() {
                    self.insert(&text);
                }
            }
            Event::Key(key) if key.kind == crate::event::KeyEventKind::Release => {
                result.handled = false
            }
            Event::Key(key) => {
                let control = key.modifiers.contains(KeyModifiers::CONTROL);
                let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                let mut target = self.cursor;
                let mut movement = true;
                match key.code {
                    KeyCode::Char('a') if control => {
                        self.anchor = Some(0);
                        self.cursor = self.value.len();
                        movement = false;
                    }
                    KeyCode::Char(c)
                        if !control
                            && !key.modifiers.contains(KeyModifiers::ALT)
                            && !c.is_control() =>
                    {
                        self.insert(c.encode_utf8(&mut [0; 4]));
                        movement = false;
                    }
                    KeyCode::Backspace => {
                        self.delete(true);
                        movement = false;
                    }
                    KeyCode::Delete => {
                        self.delete(false);
                        movement = false;
                    }
                    KeyCode::Enter => {
                        if !multiline || control {
                            result.submit = true;
                        } else {
                            self.insert("\n");
                        }
                        movement = false;
                    }
                    KeyCode::Left => {
                        target = if !shift && self.selection().is_some() {
                            self.selection().unwrap().start
                        } else {
                            previous_boundary(&self.value, self.cursor)
                        };
                    }
                    KeyCode::Right => {
                        target = if !shift && self.selection().is_some() {
                            self.selection().unwrap().end
                        } else {
                            next_boundary(&self.value, self.cursor)
                        };
                    }
                    KeyCode::Home => {
                        let (line, _) = metrics.byte_to_line_column(&self.value, self.cursor);
                        target = if multiline && !control {
                            metrics.line_column_to_byte(&self.value, line, 0)
                        } else {
                            0
                        };
                    }
                    KeyCode::End => {
                        let (line, _) = metrics.byte_to_line_column(&self.value, self.cursor);
                        target = if multiline && !control {
                            metrics.line_column_to_byte(&self.value, line, usize::MAX)
                        } else {
                            self.value.len()
                        };
                    }
                    KeyCode::Up | KeyCode::Down if multiline => {
                        let (line, col) = metrics.byte_to_line_column(&self.value, self.cursor);
                        let last_line = self.value.bytes().filter(|b| *b == b'\n').count();
                        let line = if key.code == KeyCode::Up {
                            line.saturating_sub(1)
                        } else {
                            (line + 1).min(last_line)
                        };
                        target = metrics.line_column_to_byte(&self.value, line, col);
                    }
                    _ => {
                        result.handled = false;
                        movement = false;
                    }
                }
                if movement {
                    if shift {
                        self.anchor.get_or_insert(self.cursor);
                    } else {
                        self.anchor = None;
                    }
                    self.cursor = target;
                }
            }
            _ => result.handled = false,
        }
        result.changed = self.value != original;
        result
    }
}

/// Keep the cursor inside a viewport with at least one column for its cell.
pub(crate) fn cursor_scroll(column: usize, offset: usize, width: usize) -> usize {
    if column < offset {
        column
    } else if column >= offset.saturating_add(width.max(1)) {
        column + 1 - width.max(1)
    } else {
        offset
    }
}

/// Paint an editor line with one shared column and clipping policy, including
/// the selection background on whole or partially clipped graphemes.
pub(crate) fn paint_editor_line(
    chunk: &mut render::chunk::Chunk<'_>,
    origin: (u16, u16),
    text: &str,
    byte_start: usize,
    selection: Option<std::ops::Range<usize>>,
    viewport: (usize, usize),
    style: render::style::Style,
    selection_style: render::style::Style,
) {
    let metrics = chunk.text_metrics();
    let (offset, columns) = viewport;
    let end = offset.saturating_add(columns);
    let mut column = 0;
    for (byte, g) in text.grapheme_indices(true) {
        let width = metrics.grapheme_width(g, column);
        let next = column + width;
        if column >= end {
            break;
        }
        if next > offset && width > 0 {
            let selected = selection
                .as_ref()
                .map(|s| s.contains(&(byte_start + byte)))
                .unwrap_or(false);
            let style = if selected {
                style.merge(selection_style)
            } else {
                style
            };
            let x = origin.0 + (column.max(offset) - offset) as u16;
            if g == "\t" || column < offset || next > end {
                let _ = chunk.fill(
                    x,
                    origin.1,
                    (next.min(end) - column.max(offset)) as u16,
                    1,
                    ' ',
                    style,
                );
            } else {
                let _ = chunk.set_forced(x, origin.1, render::style::Stylized::grapheme(g, style));
            }
        }
        column = next;
    }
}
