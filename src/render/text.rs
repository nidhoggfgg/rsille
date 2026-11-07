//! Shared grapheme boundaries and terminal column policy.
//!
//! Byte offsets always refer to the original UTF-8 string. Columns are zero
//! based; a column inside a wide grapheme maps to its leading byte. Controls
//! have zero width and are never sent to the terminal. Tabs advance to stops
//! relative to the start of the text line, with a default interval of four.

pub use crate::render::style::{StylizedLine as Line, StylizedText as Span};
use std::ops::Range;
pub use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Width policy for characters with East Asian Ambiguous width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AmbiguousWidth {
    /// Uses ordinary Unicode width, treating ambiguous characters as narrow.
    #[default]
    Narrow,
    /// Uses CJK width, treating ambiguous characters as wide.
    Wide,
}

/// Shared policy for measuring graphemes, expanding tabs, and mapping offsets.
///
/// Defaults to narrow ambiguous characters and tab stops every four columns.
/// Widths are terminal columns, byte offsets refer to UTF-8, and line indices
/// are zero-based. Single-line helpers ignore newline width; [`wrap`](Self::wrap)
/// and line-coordinate helpers handle explicit line boundaries.
///
/// ```
/// use rsille::render::text::TextMetrics;
///
/// let metrics = TextMetrics::default();
/// assert_eq!(metrics.width("A界"), 3);
/// assert_eq!(metrics.column_to_byte("A界", 2), 1);
/// assert_eq!(metrics.truncate("A界", 2), "A");
/// assert_eq!(metrics.expand("A\tB"), "A   B");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextMetrics {
    /// Treatment of East Asian Ambiguous characters.
    pub ambiguous_width: AmbiguousWidth,
    /// Tab-stop interval in columns; zero is treated as one.
    pub tab_width: usize,
}

impl Default for TextMetrics {
    fn default() -> Self {
        Self {
            ambiguous_width: AmbiguousWidth::Narrow,
            tab_width: 4,
        }
    }
}

impl TextMetrics {
    /// Width of a complete grapheme at the given line column.
    ///
    /// A tab advances to the next stop. Graphemes containing control characters
    /// other than a standalone tab have zero width.
    pub fn grapheme_width(self, grapheme: &str, column: usize) -> usize {
        if grapheme == "\t" {
            let interval = self.tab_width.max(1);
            return interval - column % interval;
        }
        if grapheme.chars().any(char::is_control) {
            return 0;
        }
        match self.ambiguous_width {
            AmbiguousWidth::Narrow => UnicodeWidthStr::width(grapheme),
            AmbiguousWidth::Wide => grapheme.width_cjk(),
        }
    }

    /// Display width of a single line (newlines do not add columns).
    pub fn width(self, text: &str) -> usize {
        text.graphemes(true)
            .fold(0, |column, g| column + self.grapheme_width(g, column))
    }

    /// Converts a UTF-8 byte offset to a single-line display column.
    ///
    /// Offsets inside a grapheme round down to its leading byte; offsets past
    /// the string clamp to its length. Newlines contribute no width.
    pub fn byte_to_column(self, text: &str, byte: usize) -> usize {
        self.width(&text[..floor_boundary(text, byte)])
    }

    /// Converts a display column to the leading byte of its grapheme.
    ///
    /// A column inside a wide grapheme or tab maps to its start. Columns at
    /// or beyond the end return `text.len()`; zero-width graphemes are skipped.
    pub fn column_to_byte(self, text: &str, column: usize) -> usize {
        let mut current = 0;
        for (byte, g) in text.grapheme_indices(true) {
            let width = self.grapheme_width(g, current);
            if current + width > column {
                return byte;
            }
            current += width;
        }
        text.len()
    }

    /// Prefix that fits without splitting a grapheme or tab stop.
    pub fn truncate<'a>(self, text: &'a str, columns: usize) -> &'a str {
        let mut current = 0;
        for (byte, g) in text.grapheme_indices(true) {
            let width = self.grapheme_width(g, current);
            if current + width > columns {
                return &text[..byte];
            }
            current += width;
        }
        text
    }

    /// A column viewport. Partially visible wide graphemes and tabs become
    /// spaces, preserving the positions of all following graphemes.
    /// Fully visible tabs also expand to spaces. Content beyond the source's
    /// end is not padded; controls and zero-width graphemes are omitted.
    pub fn clip(self, text: &str, start: usize, columns: usize) -> String {
        let end = start.saturating_add(columns);
        let mut column = 0;
        let mut result = String::new();
        for g in text.graphemes(true) {
            let width = self.grapheme_width(g, column);
            let next = column + width;
            if column >= end {
                break;
            }
            if next > start && width > 0 {
                if column < start || next > end || g == "\t" {
                    result.extend(std::iter::repeat_n(' ', next.min(end) - column.max(start)));
                } else {
                    result.push_str(g);
                }
            }
            column = next;
        }
        result
    }

    /// Expands tabs and removes control and zero-width graphemes.
    ///
    /// Visible non-tab graphemes are preserved. Newlines are removed, not
    /// interpreted as line breaks.
    pub fn expand(self, text: &str) -> String {
        self.clip(text, 0, usize::MAX)
    }

    /// Grapheme wrapping, returning original byte ranges and display widths.
    /// A grapheme wider than the viewport occupies its own visual line; callers
    /// clip it when painting. Explicit newlines always start another line.
    ///
    /// Both LF and CRLF delimiters are excluded from the returned ranges.
    /// Trailing newlines produce a final empty line. A zero-column viewport
    /// returns no lines; otherwise empty input produces one empty line.
    pub fn wrap(self, text: &str, columns: usize) -> Vec<VisualLine> {
        if columns == 0 {
            return Vec::new();
        }
        let mut lines = Vec::new();
        let mut start = 0;
        let mut column = 0;
        for (byte, g) in text.grapheme_indices(true) {
            if g == "\n" || g == "\r\n" {
                lines.push(VisualLine {
                    bytes: start..byte,
                    width: column,
                });
                start = byte + g.len();
                column = 0;
                continue;
            }
            let mut width = self.grapheme_width(g, column);
            if column > 0 && column + width > columns {
                lines.push(VisualLine {
                    bytes: start..byte,
                    width: column,
                });
                start = byte;
                column = 0;
                width = self.grapheme_width(g, 0);
            }
            column += width;
        }
        lines.push(VisualLine {
            bytes: start..text.len(),
            width: column,
        });
        lines
    }

    /// Maps a byte offset to a zero-based source `(line, column)` pair.
    ///
    /// Rounds down to a grapheme boundary and clamps at the end of the string.
    /// Lines are separated by LF characters, and columns use this width policy.
    /// This does not account for visual wrapping.
    pub fn byte_to_line_column(self, text: &str, byte: usize) -> (usize, usize) {
        let byte = floor_boundary(text, byte);
        let before = &text[..byte];
        let line = before.bytes().filter(|b| *b == b'\n').count();
        let start = before.rfind('\n').map_or(0, |i| i + 1);
        (line, self.width(&text[start..byte]))
    }

    /// Maps a zero-based source line and display column to a UTF-8 byte offset.
    ///
    /// Lines are separated by LF characters. Columns inside wide graphemes or
    /// tabs map to their leading byte. Columns past a line clamp to its end;
    /// line indices past the source return `text.len()`.
    pub fn line_column_to_byte(self, text: &str, line: usize, column: usize) -> usize {
        let mut start = 0;
        for (index, value) in text.split('\n').enumerate() {
            if index == line {
                return start + self.column_to_byte(value, column);
            }
            start += value.len() + 1;
        }
        text.len()
    }
}

/// A visual line produced by [`TextMetrics::wrap`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualLine {
    /// Grapheme-aligned UTF-8 range in the original text, excluding line breaks.
    pub bytes: Range<usize>,
    /// Display width in terminal columns, which can exceed the wrap width.
    pub width: usize,
}

/// Returns the nearest grapheme boundary at or before `byte`.
///
/// Offsets past the string return `text.len()`; arbitrary byte offsets are safe.
pub fn floor_boundary(text: &str, byte: usize) -> usize {
    if byte >= text.len() {
        return text.len();
    }
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|i| *i <= byte)
        .last()
        .unwrap_or(0)
}

/// Returns the nearest grapheme boundary at or after `byte`.
///
/// Returns `text.len()` when no later boundary exists.
pub fn ceil_boundary(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .find(|i| *i >= byte)
        .unwrap_or(text.len())
}

/// Returns the grapheme start strictly before `byte`, or zero if none exists.
///
/// An offset past the string returns the start of its final grapheme.
pub fn previous_boundary(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|i| *i < byte)
        .last()
        .unwrap_or(0)
}

/// Returns the grapheme boundary strictly after `byte`, or `text.len()`.
pub fn next_boundary(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .find(|i| *i > byte)
        .unwrap_or(text.len())
}

/// Rich text with explicit source lines. Span boundaries do not create new
/// graphemes: the style of the first byte owns a cluster crossing two spans.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Text<'a> {
    /// Explicit source lines, each containing zero or more styled spans.
    pub lines: Vec<Line<'a>>,
}

impl<'a> Text<'a> {
    /// Collects source lines without splitting or wrapping their contents.
    ///
    /// Use `Text::from(&str)` to split a string on LF characters, preserving
    /// empty and trailing lines.
    pub fn new(lines: impl IntoIterator<Item = Line<'a>>) -> Self {
        Self {
            lines: lines.into_iter().collect(),
        }
    }
    /// Returns the widest source line in columns, or zero if there are no lines.
    pub fn width(&self, metrics: TextMetrics) -> usize {
        self.lines
            .iter()
            .map(|l| l.width_with(metrics))
            .max()
            .unwrap_or(0)
    }
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value.split('\n').map(Line::from))
    }
}

/// Convenience width measurement using the default terminal policy.
pub trait TextWidth {
    /// Measures a single line with narrow ambiguous width and four-column tabs.
    fn width(&self) -> usize;
}
impl TextWidth for str {
    fn width(&self) -> usize {
        TextMetrics::default().width(self)
    }
}
