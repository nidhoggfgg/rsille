//! Colors, attributes, styled graphemes, and rich text spans.
//!
//! [`Stylized`] stores one grapheme, [`StylizedText`] stores a styled UTF-8 span,
//! and [`StylizedLine`] joins spans before segmentation so a style boundary
//! cannot split a grapheme. Widths are measured in terminal columns.

use std::{borrow::Cow, io};

use crate::render::text::{AmbiguousWidth, TextMetrics, UnicodeSegmentation};
use crossterm::style::{Attributes, Colors, Print, SetAttributes, SetColors};
use std::sync::Arc;

/// Optional terminal colors and text attributes.
///
/// The default has neither. Missing settings can inherit through
/// [`merge`](Self::merge); queueing this style alone does not reset terminal
/// settings left by earlier output.
#[derive(Debug, Clone, PartialEq, Eq, Default, Copy)]
pub struct Style {
    /// Optional foreground and background color settings.
    pub colors: Option<Colors>,
    /// Optional text attributes, such as bold or underline.
    pub attr: Option<Attributes>,
}

impl Style {
    /// Creates a style without explicit colors or attributes.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a style with colors and no explicit attributes.
    #[inline]
    pub fn with_colors(colors: Colors) -> Self {
        Self {
            colors: Some(colors),
            attr: None,
        }
    }

    /// Creates a style with attributes and no explicit colors.
    #[inline]
    pub fn with_attr(attr: Attributes) -> Self {
        Self {
            colors: None,
            attr: Some(attr),
        }
    }

    /// Creates a style with both colors and attributes.
    #[inline]
    pub fn with_both(colors: Colors, attr: Attributes) -> Self {
        Self {
            colors: Some(colors),
            attr: Some(attr),
        }
    }

    /// Replaces the color settings and returns the updated style.
    #[inline]
    pub fn colors(mut self, colors: Colors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Replaces the attributes and returns the updated style.
    #[inline]
    pub fn attr(mut self, attr: Attributes) -> Self {
        self.attr = Some(attr);
        self
    }

    /// Replaces the color settings in place and returns this style.
    #[inline]
    pub fn set_colors(&mut self, colors: Colors) -> &mut Self {
        self.colors = Some(colors);
        self
    }

    /// Replaces the attributes in place and returns this style.
    #[inline]
    pub fn set_attr(&mut self, attr: Attributes) -> &mut Self {
        self.attr = Some(attr);
        self
    }

    /// Returns whether color settings are explicitly present.
    #[inline]
    pub fn has_color(&self) -> bool {
        self.colors.is_some()
    }

    /// Returns whether attributes are explicitly present, even if empty.
    #[inline]
    pub fn has_attr(&self) -> bool {
        self.attr.is_some()
    }

    /// Overlays `other`, inheriting any unspecified settings from this style.
    ///
    /// Foreground and background colors merge independently. An explicit
    /// attribute set replaces the old set rather than adding to it.
    #[inline]
    pub fn merge(self, other: Style) -> Self {
        Self {
            colors: merge_colors(self.colors, other.colors),
            attr: other.attr.or(self.attr),
        }
    }
}

#[inline]
fn merge_colors(base: Option<Colors>, overlay: Option<Colors>) -> Option<Colors> {
    match (base, overlay) {
        (Some(base), Some(overlay)) => Some(base.then(&overlay)),
        (Some(base), None) => Some(base),
        (None, Some(overlay)) => Some(overlay),
        (None, None) => None,
    }
}

/// A terminal cell containing one extended grapheme cluster.
/// Scalars are stored inline; multi-scalar clusters share their UTF-8 storage
/// when buffers are cloned. Empty cells produce no terminal output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stylized {
    symbol: Symbol,
    /// Colors and attributes applied when the grapheme is rendered.
    pub style: Style,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Symbol {
    #[default]
    Empty,
    Scalar(char),
    Cluster(Arc<str>),
}

impl Stylized {
    /// Creates a styled Unicode scalar; use [`grapheme`](Self::grapheme) for clusters.
    pub fn new(c: char, style: Style) -> Self {
        Self {
            symbol: Symbol::Scalar(c),
            style,
        }
    }
    /// Creates a scalar with the default style.
    pub fn plain(c: char) -> Self {
        Self::new(c, Style::default())
    }
    /// Creates an unstyled space occupying one column.
    pub fn space() -> Self {
        Self::plain(' ')
    }
    /// Creates an empty, zero-width cell with the default style.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Stores only the first complete grapheme in `text`, preserving `style`.
    ///
    /// Empty input creates an empty cell. Use
    /// [`Chunk::set_string`](crate::render::chunk::Chunk::set_string) for text
    /// containing multiple graphemes.
    pub fn grapheme(text: &str, style: Style) -> Self {
        let symbol = match text.graphemes(true).next() {
            None => Symbol::Empty,
            Some(g) => {
                let mut chars = g.chars();
                let first = chars.next().unwrap();
                if chars.next().is_none() {
                    Symbol::Scalar(first)
                } else {
                    Symbol::Cluster(Arc::from(g))
                }
            }
        };
        Self { symbol, style }
    }

    /// Returns the symbol as UTF-8, allocating only for a scalar character.
    pub fn symbol(&self) -> Cow<'_, str> {
        match &self.symbol {
            Symbol::Empty => Cow::Borrowed(""),
            Symbol::Scalar(c) => Cow::Owned(c.to_string()),
            Symbol::Cluster(text) => Cow::Borrowed(text),
        }
    }

    /// Scalar-only compatibility accessor. Multi-scalar graphemes return None.
    pub fn as_char(&self) -> Option<char> {
        match self.symbol {
            Symbol::Scalar(c) => Some(c),
            _ => None,
        }
    }
    /// Replaces the symbol with a scalar, preserving its style.
    pub fn set_char(&mut self, c: char) -> &mut Self {
        self.symbol = Symbol::Scalar(c);
        self
    }
    /// Replaces the symbol with the first grapheme of `text`, preserving style.
    ///
    /// Empty input clears the symbol.
    pub fn set_grapheme(&mut self, text: &str) -> &mut Self {
        self.symbol = Self::grapheme(text, self.style).symbol;
        self
    }
    /// Replaces the style in place without changing the symbol.
    pub fn set_style(&mut self, style: Style) -> &mut Self {
        self.style = style;
        self
    }
    /// Replaces the style and returns the updated cell.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    /// Returns whether no symbol is stored; a space is not empty.
    pub fn is_empty(&self) -> bool {
        matches!(self.symbol, Symbol::Empty)
    }
    /// Returns whether the symbol is empty or contains only Unicode whitespace.
    pub fn is_whitespace(&self) -> bool {
        match &self.symbol {
            Symbol::Empty => true,
            Symbol::Scalar(c) => c.is_whitespace(),
            Symbol::Cluster(text) => text.chars().all(char::is_whitespace),
        }
    }
    /// Returns whether the style has explicit color settings.
    pub fn has_color(&self) -> bool {
        self.style.has_color()
    }
    /// Returns whether the style has explicit attributes.
    pub fn has_attr(&self) -> bool {
        self.style.has_attr()
    }
    /// Measures the symbol with default text metrics at column zero.
    pub fn width(&self) -> usize {
        self.width_with(TextMetrics::default(), 0)
    }
    /// Measures at column zero with ambiguous characters treated as wide.
    pub fn width_cjk(&self) -> usize {
        self.width_with(
            TextMetrics {
                ambiguous_width: AmbiguousWidth::Wide,
                ..TextMetrics::default()
            },
            0,
        )
    }
    /// Measures the symbol using `metrics` at the given tab-stop column.
    ///
    /// Empty symbols and control graphemes have zero width, except tabs,
    /// which advance to the next configured stop.
    pub fn width_with(&self, metrics: TextMetrics, column: usize) -> usize {
        // Avoid allocation for the overwhelmingly common scalar cell.
        let mut bytes = [0; 4];
        match &self.symbol {
            Symbol::Empty => 0,
            Symbol::Scalar(c) => metrics.grapheme_width(c.encode_utf8(&mut bytes), column),
            Symbol::Cluster(text) => metrics.grapheme_width(text, column),
        }
    }
    /// Queues style commands and the symbol without flushing the writer.
    ///
    /// Empty and zero-width symbols produce no output. Tabs become four
    /// spaces using default metrics at column zero. Missing colors and
    /// attributes do not reset earlier terminal styling.
    ///
    /// # Errors
    ///
    /// Propagates errors from writing terminal commands or symbol content.
    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        if self.is_empty() || self.width() == 0 {
            return Ok(());
        }
        if let Some(colors) = self.style.colors {
            crossterm::queue!(buffer, SetColors(colors))?;
        }
        if let Some(attr) = self.style.attr {
            crossterm::queue!(buffer, SetAttributes(attr))?;
        }
        match &self.symbol {
            Symbol::Empty => {}
            Symbol::Scalar('\t') => {
                crossterm::queue!(buffer, Print(" ".repeat(self.width())))?;
            }
            Symbol::Scalar(c) => {
                crossterm::queue!(buffer, Print(c))?;
            }
            Symbol::Cluster(text) => {
                crossterm::queue!(buffer, Print(text))?;
            }
        }
        Ok(())
    }
}

impl From<char> for Stylized {
    #[inline]
    fn from(c: char) -> Self {
        Self::plain(c)
    }
}

/// A single logical line made of styled spans.
///
/// Measurement and cell conversion segment the concatenated text, preserving
/// graphemes across span boundaries. The leading span supplies a crossing
/// grapheme's style. Embedded newlines do not create additional lines here.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StylizedLine<'a> {
    /// Ordered spans whose text is concatenated to form the line.
    pub content: Vec<StylizedText<'a>>,
}

impl<'a> StylizedLine<'a> {
    /// Creates a line with no spans.
    #[inline]
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }

    /// Creates a line containing one span.
    #[inline]
    pub fn from_text(text: StylizedText<'a>) -> Self {
        Self {
            content: vec![text],
        }
    }

    /// Creates a line from ordered spans.
    #[inline]
    pub fn from_texts(content: Vec<StylizedText<'a>>) -> Self {
        Self { content }
    }

    /// Creates a line containing one styled string, borrowed or owned.
    #[inline]
    pub fn styled<T>(content: T, style: Style) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self {
            content: vec![StylizedText::new(content, style)],
        }
    }

    /// Appends a span and returns this line.
    #[inline]
    pub fn push(&mut self, text: StylizedText<'a>) -> &mut Self {
        self.content.push(text);
        self
    }

    /// Measures the concatenated line using default text metrics.
    #[inline]
    pub fn width(&self) -> usize {
        self.width_with(TextMetrics::default())
    }

    /// Measures the concatenated line using the supplied width and tab policy.
    pub fn width_with(&self, metrics: TextMetrics) -> usize {
        metrics.width(
            &self
                .content
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>(),
        )
    }

    /// Collects complete graphemes, using the leading span's style for each.
    ///
    /// Segments the concatenated text, including zero-width graphemes and tabs.
    pub fn cells(&self) -> Vec<Stylized> {
        let text: String = self.content.iter().map(|s| s.content.as_ref()).collect();
        let mut offset = 0;
        let spans: Vec<_> = self
            .content
            .iter()
            .map(|span| {
                offset += span.content.len();
                (offset, span.style)
            })
            .collect();
        text.grapheme_indices(true)
            .map(|(byte, g)| {
                let style = spans
                    .iter()
                    .find(|(end, _)| *end > byte)
                    .map(|(_, s)| *s)
                    .unwrap_or_default();
                Stylized::grapheme(g, style)
            })
            .collect()
    }

    /// Returns whether the line has no spans, regardless of their text contents.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Returns the number of spans, not the number of graphemes or columns.
    #[inline]
    pub fn len(&self) -> usize {
        self.content.len()
    }
}

impl<'a> From<&'a str> for StylizedLine<'a> {
    #[inline]
    fn from(s: &'a str) -> Self {
        Self {
            content: vec![StylizedText::from(s)],
        }
    }
}

impl From<String> for StylizedLine<'_> {
    #[inline]
    fn from(s: String) -> Self {
        Self {
            content: vec![StylizedText::from(s)],
        }
    }
}

impl<'a> From<Cow<'a, str>> for StylizedLine<'a> {
    #[inline]
    fn from(s: Cow<'a, str>) -> Self {
        Self {
            content: vec![StylizedText::from(s)],
        }
    }
}

impl<'a> From<Vec<StylizedText<'a>>> for StylizedLine<'a> {
    #[inline]
    fn from(content: Vec<StylizedText<'a>>) -> Self {
        Self { content }
    }
}

impl<'a> From<StylizedText<'a>> for StylizedLine<'a> {
    #[inline]
    fn from(text: StylizedText<'a>) -> Self {
        Self::from_text(text)
    }
}

/// A borrowed or owned UTF-8 text span with a single style.
///
/// Iterating over `&StylizedText` yields [`Stylized`] graphemes with this style.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StylizedText<'a> {
    /// UTF-8 text, borrowed from the caller or owned by this span.
    pub content: Cow<'a, str>,
    /// Colors and attributes shared by all graphemes in the span.
    pub style: Style,
}

impl<'a> StylizedText<'a> {
    /// Creates a styled span from borrowed or owned text.
    #[inline]
    pub fn new<T>(content: T, style: Style) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self {
            content: content.into(),
            style,
        }
    }

    /// Creates a span with the default style.
    #[inline]
    pub fn plain<T>(content: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self {
            content: content.into(),
            style: Style::default(),
        }
    }

    /// Replaces the span's style in place and returns this span.
    #[inline]
    pub fn set_style(&mut self, style: Style) -> &mut Self {
        self.style = style;
        self
    }

    /// Replaces the style and returns the updated span.
    #[inline]
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Measures the text using default text metrics.
    #[inline]
    pub fn width(&self) -> usize {
        self.width_with(TextMetrics::default())
    }

    /// Measures the text as a single line using the supplied width and tab policy.
    pub fn width_with(&self, metrics: TextMetrics) -> usize {
        metrics.width(&self.content)
    }

    /// Measures the text with ambiguous characters treated as wide.
    #[inline]
    pub fn width_cjk(&self) -> usize {
        TextMetrics {
            ambiguous_width: AmbiguousWidth::Wide,
            ..TextMetrics::default()
        }
        .width(&self.content)
    }

    /// Returns whether the UTF-8 text is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Returns the number of extended grapheme clusters, not bytes or columns.
    #[inline]
    pub fn len(&self) -> usize {
        self.content.graphemes(true).count()
    }

    /// Queues the span without flushing, propagating writer errors.
    ///
    /// Uses default text metrics to expand tabs and remove control and
    /// zero-width graphemes. Newlines are removed rather than moving the cursor.
    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        let text = TextMetrics::default().expand(&self.content);
        for c in text.graphemes(true) {
            Stylized::grapheme(c, self.style).queue(buffer)?;
        }
        Ok(())
    }
}

/// Styled graphemes yielded by iterating over a borrowed [`StylizedText`].
///
/// Each item retains the span's style; tabs and controls are not expanded here.
pub struct StylizedTextIter<'a> {
    graphemes: unicode_segmentation::Graphemes<'a>,
    style: Style,
}

impl<'a> StylizedTextIter<'a> {
    fn new(text: &'a StylizedText<'a>) -> Self {
        Self {
            graphemes: text.content.graphemes(true),
            style: text.style,
        }
    }
}

impl Iterator for StylizedTextIter<'_> {
    type Item = Stylized;

    fn next(&mut self) -> Option<Self::Item> {
        let c = self.graphemes.next()?;
        Some(Stylized::grapheme(c, self.style))
    }
}

impl<'a> IntoIterator for &'a StylizedText<'a> {
    type Item = Stylized;
    type IntoIter = StylizedTextIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        StylizedTextIter::new(self)
    }
}

impl<'a> From<&'a str> for StylizedText<'a> {
    #[inline]
    fn from(s: &'a str) -> Self {
        Self::plain(Cow::Borrowed(s))
    }
}

impl From<String> for StylizedText<'_> {
    #[inline]
    fn from(s: String) -> Self {
        Self::plain(Cow::Owned(s))
    }
}

impl<'a> From<Cow<'a, str>> for StylizedText<'a> {
    #[inline]
    fn from(s: Cow<'a, str>) -> Self {
        Self::plain(s)
    }
}
