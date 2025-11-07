//! Styling types for widget appearance

mod border;
mod css;
mod error;
mod padding;

pub use border::BorderStyle;
pub use padding::Padding;

pub use error::CssError;

/// Widget style configuration
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
    pub modifiers: TextModifiers,
    pub border: Option<BorderStyle>,
    pub padding: Padding,
}

impl Style {
    pub fn default() -> Self {
        Self {
            fg_color: None,
            bg_color: None,
            modifiers: TextModifiers::empty(),
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

    /// Convert to render style
    pub fn to_render_style(&self) -> render::style::Style {
        use crossterm::style::{Attributes, Colors};
        use render::style::Style as RenderStyle;

        // Convert colors if present
        let colors = if self.fg_color.is_some() || self.bg_color.is_some() {
            let fg = self.fg_color.map(color_to_crossterm);
            let bg = self.bg_color.map(color_to_crossterm);
            Some(Colors {
                foreground: fg,
                background: bg,
            })
        } else {
            None
        };

        // Convert modifiers if present
        let attr = if !self.modifiers.is_empty() {
            let mut a = Attributes::default();
            if self.modifiers.contains_bold() {
                a = a | crossterm::style::Attribute::Bold;
            }
            if self.modifiers.contains_italic() {
                a = a | crossterm::style::Attribute::Italic;
            }
            if self.modifiers.contains_underlined() {
                a = a | crossterm::style::Attribute::Underlined;
            }
            Some(a)
        } else {
            None
        };

        // Create render style
        match (colors, attr) {
            (Some(c), Some(a)) => RenderStyle::with_both(c, a),
            (Some(c), None) => RenderStyle::with_colors(c),
            (None, Some(a)) => RenderStyle::with_attr(a),
            (None, None) => RenderStyle::default(),
        }
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
pub struct TextModifiers {
    bits: u8,
}

impl TextModifiers {
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

/// Convert TUI Color to crossterm Color
fn color_to_crossterm(color: Color) -> crossterm::style::Color {
    use crossterm::style::Color as CC;
    match color {
        Color::Black => CC::Black,
        Color::Red => CC::DarkRed,
        Color::Green => CC::DarkGreen,
        Color::Yellow => CC::DarkYellow,
        Color::Blue => CC::DarkBlue,
        Color::Magenta => CC::DarkMagenta,
        Color::Cyan => CC::DarkCyan,
        Color::White => CC::White,
        Color::Rgb(r, g, b) => CC::Rgb { r, g, b },
        Color::Indexed(i) => CC::AnsiValue(i),
    }
}
