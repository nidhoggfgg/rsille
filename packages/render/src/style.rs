use std::{borrow::Cow, io};

use term::crossterm::{
    queue,
    style::{Attributes, Colors, Print, SetAttributes, SetColors},
};
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone, PartialEq, Eq, Default, Copy)]
pub struct Style {
    pub colors: Option<Colors>,
    pub attr: Option<Attributes>,
}

impl Style {
    pub fn new(colors: Option<Colors>, attr: Option<Attributes>) -> Self {
        Self { colors, attr }
    }

    pub fn set_colors(&mut self, colors: Colors) {
        self.colors = Some(colors);
    }

    pub fn set_attr(&mut self, attr: Attributes) {
        self.attr = Some(attr);
    }

    pub fn has_color(&self) -> bool {
        self.colors.is_some()
    }

    pub fn has_attr(&self) -> bool {
        self.attr.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Copy)]
pub struct Stylized {
    pub c: Option<char>,
    pub style: Style,
}

impl Stylized {
    pub fn new(c: char, style: Style) -> Self {
        Self { c: Some(c), style }
    }

    pub fn raw(c: char) -> Self {
        Self {
            c: Some(c),
            style: Default::default(),
        }
    }

    pub fn set_char(&mut self, c: char) {
        self.c = Some(c);
    }

    pub fn has_color(&self) -> bool {
        self.style.has_color()
    }

    pub fn has_attr(&self) -> bool {
        self.style.has_attr()
    }

    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        if let Some(c) = self.c {
            if let Some(c) = self.style.colors {
                queue!(buffer, SetColors(c))?
            }
            if let Some(a) = self.style.attr {
                queue!(buffer, SetAttributes(a))?
            }
            queue!(buffer, Print(c))?;
        }
        Ok(())
    }

    pub fn space() -> Self {
        Self {
            c: Some(' '),
            style: Default::default(),
        }
    }

    pub fn nop() -> Self {
        Self {
            c: None,
            style: Default::default(),
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        if let Some(c) = self.c {
            return c.width().unwrap_or(0);
        }
        0
    }

    #[inline]
    pub fn width_cjk(&self) -> usize {
        if let Some(c) = self.c {
            return c.width_cjk().unwrap_or(0);
        }
        0
    }
}

impl From<char> for Stylized {
    fn from(c: char) -> Self {
        Self::new(c, Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StylizedLine<'a> {
    pub content: Vec<StylizedText<'a>>,
}

impl<'a> StylizedLine<'a> {
    pub fn width(&self) -> usize {
        self.content.iter().map(|x| x.width()).sum()
    }

    pub fn new<T>(content: T, style: Style) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self {
            content: vec![StylizedText::new(content, style)],
        }
    }
}

impl<'a> From<&'a str> for StylizedLine<'a> {
    fn from(s: &'a str) -> Self {
        Self {
            content: vec![StylizedText::from(s)],
        }
    }
}

impl From<String> for StylizedLine<'_> {
    fn from(s: String) -> Self {
        Self {
            content: vec![StylizedText::from(s)],
        }
    }
}

impl<'a> From<Cow<'a, str>> for StylizedLine<'a> {
    fn from(s: Cow<'a, str>) -> Self {
        Self {
            content: vec![StylizedText::from(s)],
        }
    }
}

impl<'a> From<Vec<StylizedText<'a>>> for StylizedLine<'a> {
    fn from(content: Vec<StylizedText<'a>>) -> Self {
        Self { content }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StylizedText<'a> {
    pub content: Cow<'a, str>,
    pub style: Style,
}

impl<'a> StylizedText<'a> {
    pub fn new<T>(content: T, style: Style) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self {
            content: content.into(),
            style,
        }
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        for c in self.content.chars() {
            Stylized::new(c, self.style).queue(buffer)?;
        }
        Ok(())
    }

    pub fn width(&self) -> usize {
        self.content.chars().map(|x| x.width().unwrap_or(0)).sum()
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
    fn from(s: &'a str) -> Self {
        Self::new(Cow::Borrowed(s), Default::default())
    }
}

impl From<String> for StylizedText<'_> {
    fn from(s: String) -> Self {
        Self::new(Cow::Owned(s), Default::default())
    }
}

impl<'a> From<Cow<'a, str>> for StylizedText<'a> {
    fn from(s: Cow<'a, str>) -> Self {
        Self::new(s, Default::default())
    }
}
