//! Styling types for widget appearance

pub mod border;
pub mod css;
pub mod padding;

pub use border::BorderStyle;
pub use css::CssError;
pub use padding::Padding;

/// Widget style configuration
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
    pub modifiers: Modifiers,
    pub border: Option<BorderStyle>,
    pub padding: Padding,
}

impl Style {
    pub fn default() -> Self {
        Self {
            fg_color: None,
            bg_color: None,
            modifiers: Modifiers::empty(),
            border: None,
            padding: Padding::ZERO,
        }
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg_color = Some(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.modifiers = self.modifiers.with_bold();
        self
    }

    pub fn italic(mut self) -> Self {
        self.modifiers = self.modifiers.with_italic();
        self
    }

    pub fn underlined(mut self) -> Self {
        self.modifiers = self.modifiers.with_underlined();
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
}

/// Terminal colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    /// RGB color (true color)
    Rgb(u8, u8, u8),
    /// 256-color palette index
    Indexed(u8),
}

/// Text modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
    bits: u8,
}

impl Modifiers {
    const BOLD: u8 = 0b00001;
    const DIM: u8 = 0b00010;
    const ITALIC: u8 = 0b00100;
    const UNDERLINED: u8 = 0b01000;
    const REVERSED: u8 = 0b10000;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn with_bold(mut self) -> Self {
        self.bits |= Self::BOLD;
        self
    }

    pub const fn with_dim(mut self) -> Self {
        self.bits |= Self::DIM;
        self
    }

    pub const fn with_italic(mut self) -> Self {
        self.bits |= Self::ITALIC;
        self
    }

    pub const fn with_underlined(mut self) -> Self {
        self.bits |= Self::UNDERLINED;
        self
    }

    pub const fn with_reversed(mut self) -> Self {
        self.bits |= Self::REVERSED;
        self
    }

    pub const fn contains_bold(&self) -> bool {
        self.bits & Self::BOLD != 0
    }

    pub const fn contains_italic(&self) -> bool {
        self.bits & Self::ITALIC != 0
    }

    pub const fn contains_underlined(&self) -> bool {
        self.bits & Self::UNDERLINED != 0
    }

    pub const fn is_empty(&self) -> bool {
        self.bits == 0
    }
}
