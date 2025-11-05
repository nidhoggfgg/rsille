use std::{borrow::Cow, io};

use crossterm::{
    queue,
    style::{Attributes, Colors, Print, SetAttributes, SetColors},
};
use unicode_width::UnicodeWidthChar;

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
            colors: other.colors.or(self.colors),
            attr: other.attr.or(self.attr),
        }
    }
}

/// 带样式的单个字符
#[derive(Debug, Clone, PartialEq, Eq, Default, Copy)]
pub struct Stylized {
    pub c: Option<char>,
    pub style: Style,
}

impl Stylized {
    /// 创建一个带样式的字符
    #[inline]
    pub fn new(c: char, style: Style) -> Self {
        Self { c: Some(c), style }
    }

    /// 创建一个无样式的字符
    #[inline]
    pub fn plain(c: char) -> Self {
        Self {
            c: Some(c),
            style: Style::default(),
        }
    }

    /// 创建一个空格字符
    #[inline]
    pub fn space() -> Self {
        Self::plain(' ')
    }

    /// 创建一个空字符（不渲染任何内容）
    #[inline]
    pub fn empty() -> Self {
        Self {
            c: None,
            style: Style::default(),
        }
    }

    /// 设置字符
    #[inline]
    pub fn set_char(&mut self, c: char) -> &mut Self {
        self.c = Some(c);
        self
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

    /// 是否为空字符
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.c.is_none()
    }

    /// 检查是否有颜色样式
    #[inline]
    pub fn has_color(&self) -> bool {
        self.style.has_color()
    }

    /// 检查是否有属性样式
    #[inline]
    pub fn has_attr(&self) -> bool {
        self.style.has_attr()
    }

    /// 获取字符宽度
    #[inline]
    pub fn width(&self) -> usize {
        self.c.and_then(|c| c.width()).unwrap_or(0)
    }

    /// 获取字符宽度（CJK 模式）
    #[inline]
    pub fn width_cjk(&self) -> usize {
        self.c.and_then(|c| c.width_cjk()).unwrap_or(0)
    }

    /// 将字符和样式输出到缓冲区
    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        if let Some(ch) = self.c {
            if let Some(colors) = self.style.colors {
                queue!(buffer, SetColors(colors))?;
            }
            if let Some(attr) = self.style.attr {
                queue!(buffer, SetAttributes(attr))?;
            }
            queue!(buffer, Print(ch))?;
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
        self.content.iter().map(|x| x.width()).sum()
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
        self.content.chars().map(|x| x.width().unwrap_or(0)).sum()
    }

    /// 获取文本显示宽度（CJK 模式）
    #[inline]
    pub fn width_cjk(&self) -> usize {
        self.content
            .chars()
            .map(|x| x.width_cjk().unwrap_or(0))
            .sum()
    }

    /// 判断是否为空文本
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// 获取文本长度（字符数）
    #[inline]
    pub fn len(&self) -> usize {
        self.content.chars().count()
    }

    /// 将文本输出到缓冲区
    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        for c in self.content.chars() {
            Stylized::new(c, self.style).queue(buffer)?;
        }
        Ok(())
    }
}

pub struct StylizedTextIter<'a> {
    chars: std::str::Chars<'a>,
    style: Style,
}

impl<'a> StylizedTextIter<'a> {
    fn new(text: &'a StylizedText<'a>) -> Self {
        Self {
            chars: text.content.chars(),
            style: text.style,
        }
    }
}

impl Iterator for StylizedTextIter<'_> {
    type Item = Stylized;

    fn next(&mut self) -> Option<Self::Item> {
        let c = self.chars.next()?;
        Some(Stylized::new(c, self.style))
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
