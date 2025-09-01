use term::crossterm::style::Color;

use crate::style::Border;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Style {
    pub display: StyleDisplay,
    pub width: ElementSize,
    pub height: ElementSize,
    pub float: bool,
    pub color: Option<Color>,            // Foreground color
    pub background_color: Option<Color>, // Background color
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub text_decoration: TextDecoration,
    pub padding: Padding,
    pub margin: Margin,
    pub border: Option<Border>, // Integrate from border.rs
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    // Add more like Overline, LineThrough
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct Padding {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct Margin {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub enum StyleDisplay {
    #[default]
    Block,
    Inline,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub enum ElementSize {
    #[default]
    Auto,
    Fixed(u16),
    // Could add Percent(f32) for more CSS-like
}
