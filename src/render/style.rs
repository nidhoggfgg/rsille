use std::{borrow::Cow, io};

use crate::render::text::{AmbiguousWidth, TextMetrics, UnicodeSegmentation};
use crossterm::style::{Attributes, Colors, Print, SetAttributes, SetColors};
use std::sync::Arc;

/// 样式定义，包含颜色和文本属性
#[derive(Debug, Clone, PartialEq, Eq, Default, Copy)]
pub struct Style {
    pub colors: Option<Colors>,
    pub attr: Option<Attributes>,
}

impl Style {
    /// 创建一个新的空样式
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带有颜色的样式
    #[inline]
    pub fn with_colors(colors: Colors) -> Self {
        Self {
            colors: Some(colors),
            attr: None,
        }
    }

    /// 创建带有属性的样式
    #[inline]
    pub fn with_attr(attr: Attributes) -> Self {
        Self {
            colors: None,
            attr: Some(attr),
        }
    }

    /// 创建完整的样式
    #[inline]
    pub fn with_both(colors: Colors, attr: Attributes) -> Self {
        Self {
            colors: Some(colors),
            attr: Some(attr),
        }
    }

    /// 设置颜色（链式调用）
    #[inline]
    pub fn colors(mut self, colors: Colors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// 设置属性（链式调用）
    #[inline]
    pub fn attr(mut self, attr: Attributes) -> Self {
        self.attr = Some(attr);
        self
    }

    /// 可变设置颜色
    #[inline]
    pub fn set_colors(&mut self, colors: Colors) -> &mut Self {
        self.colors = Some(colors);
        self
    }

    /// 可变设置属性
    #[inline]
    pub fn set_attr(&mut self, attr: Attributes) -> &mut Self {
        self.attr = Some(attr);
        self
    }

    /// 检查是否有颜色
    #[inline]
    pub fn has_color(&self) -> bool {
        self.colors.is_some()
    }

    /// 检查是否有属性
    #[inline]
    pub fn has_attr(&self) -> bool {
        self.attr.is_some()
    }

    /// 合并另一个样式（other 的设置会覆盖当前设置）
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
    pub fn new(c: char, style: Style) -> Self {
        Self {
            symbol: Symbol::Scalar(c),
            style,
        }
    }
    pub fn plain(c: char) -> Self {
        Self::new(c, Style::default())
    }
    pub fn space() -> Self {
        Self::plain(' ')
    }
    pub fn empty() -> Self {
        Self::default()
    }

    /// Store the first complete grapheme; use Chunk::set_string for text.
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
    pub fn set_char(&mut self, c: char) -> &mut Self {
        self.symbol = Symbol::Scalar(c);
        self
    }
    pub fn set_grapheme(&mut self, text: &str) -> &mut Self {
        self.symbol = Self::grapheme(text, self.style).symbol;
        self
    }
    pub fn set_style(&mut self, style: Style) -> &mut Self {
        self.style = style;
        self
    }
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn is_empty(&self) -> bool {
        matches!(self.symbol, Symbol::Empty)
    }
    pub fn is_whitespace(&self) -> bool {
        match &self.symbol {
            Symbol::Empty => true,
            Symbol::Scalar(c) => c.is_whitespace(),
            Symbol::Cluster(text) => text.chars().all(char::is_whitespace),
        }
    }
    pub fn has_color(&self) -> bool {
        self.style.has_color()
    }
    pub fn has_attr(&self) -> bool {
        self.style.has_attr()
    }
    pub fn width(&self) -> usize {
        self.width_with(TextMetrics::default(), 0)
    }
    pub fn width_cjk(&self) -> usize {
        self.width_with(
            TextMetrics {
                ambiguous_width: AmbiguousWidth::Wide,
                ..TextMetrics::default()
            },
            0,
        )
    }
    pub fn width_with(&self, metrics: TextMetrics, column: usize) -> usize {
        // Avoid allocation for the overwhelmingly common scalar cell.
        let mut bytes = [0; 4];
        match &self.symbol {
            Symbol::Empty => 0,
            Symbol::Scalar(c) => metrics.grapheme_width(c.encode_utf8(&mut bytes), column),
            Symbol::Cluster(text) => metrics.grapheme_width(text, column),
        }
    }
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

/// 带样式的文本行，由多个文本片段组成
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StylizedLine<'a> {
    pub content: Vec<StylizedText<'a>>,
}

impl<'a> StylizedLine<'a> {
    /// 创建一个空行
    #[inline]
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }

    /// 从单个文本片段创建行
    #[inline]
    pub fn from_text(text: StylizedText<'a>) -> Self {
        Self {
            content: vec![text],
        }
    }

    /// 从多个文本片段创建行
    #[inline]
    pub fn from_texts(content: Vec<StylizedText<'a>>) -> Self {
        Self { content }
    }

    /// 从字符串创建带样式的行
    #[inline]
    pub fn styled<T>(content: T, style: Style) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self {
            content: vec![StylizedText::new(content, style)],
        }
    }

    /// 添加文本片段
    #[inline]
    pub fn push(&mut self, text: StylizedText<'a>) -> &mut Self {
        self.content.push(text);
        self
    }

    /// 获取行的显示宽度
    #[inline]
    pub fn width(&self) -> usize {
        self.width_with(TextMetrics::default())
    }

    pub fn width_with(&self, metrics: TextMetrics) -> usize {
        metrics.width(
            &self
                .content
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>(),
        )
    }

    /// Graphemes spanning multiple spans use the leading span's style.
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

    /// 判断是否为空行
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// 获取片段数量
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

/// 带样式的文本片段
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StylizedText<'a> {
    pub content: Cow<'a, str>,
    pub style: Style,
}

impl<'a> StylizedText<'a> {
    /// 创建带样式的文本片段
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

    /// 创建无样式的文本片段
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

    /// 设置样式
    #[inline]
    pub fn set_style(&mut self, style: Style) -> &mut Self {
        self.style = style;
        self
    }

    /// 应用样式（链式调用）
    #[inline]
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// 获取文本显示宽度
    #[inline]
    pub fn width(&self) -> usize {
        self.width_with(TextMetrics::default())
    }

    pub fn width_with(&self, metrics: TextMetrics) -> usize {
        metrics.width(&self.content)
    }

    /// 获取文本显示宽度（CJK 模式）
    #[inline]
    pub fn width_cjk(&self) -> usize {
        TextMetrics {
            ambiguous_width: AmbiguousWidth::Wide,
            ..TextMetrics::default()
        }
        .width(&self.content)
    }

    /// 判断是否为空文本
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// 获取文本长度（字素簇数）
    #[inline]
    pub fn len(&self) -> usize {
        self.content.graphemes(true).count()
    }

    /// 将文本输出到缓冲区
    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        let text = TextMetrics::default().expand(&self.content);
        for c in text.graphemes(true) {
            Stylized::grapheme(c, self.style).queue(buffer)?;
        }
        Ok(())
    }
}

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
