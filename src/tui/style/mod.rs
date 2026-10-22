//! Styling types for widget appearance

mod border;
mod css;
mod error;
mod padding;
mod theme;

pub use border::{BorderChars, BorderStyle};
pub use padding::Padding;

pub use error::CssError;
pub use theme::{EffectSlot, Theme, ThemeBuilder, ThemeEffects, ThemeStyles};

/// Widget style configuration (visual properties only)
///
/// Style contains only visual/themeable properties like colors and text modifiers.
/// Layout properties like border and padding should be set directly on widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
    pub modifiers: TextModifiers,
}

impl Style {
    pub fn default() -> Self {
        Self {
            fg_color: None,
            bg_color: None,
            modifiers: TextModifiers::empty(),
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

    /// Merge this style with another, preferring this style's values when set
    ///
    /// This is useful for combining explicit widget styles with theme defaults.
    /// Values from `self` take precedence over values from `other`.
    ///
    /// # Example
    /// ```no_run
    /// use rsille::tui::style::{Style, Color};
    ///
    /// let theme_style = Style::default().fg(Color::White).bg(Color::Blue);
    /// let widget_style = Style::default().fg(Color::Red); // Override fg only
    /// let final_style = widget_style.merge(theme_style);
    /// // final_style has fg=Red (from widget_style) and bg=Blue (from theme_style)
    /// ```
    pub fn merge(self, other: Style) -> Self {
        Self {
            fg_color: self.fg_color.or(other.fg_color),
            bg_color: self.bg_color.or(other.bg_color),
            modifiers: if self.modifiers.is_empty() {
                other.modifiers
            } else {
                self.modifiers
            },
        }
    }

    /// Check if this style has any values set
    pub fn is_empty(&self) -> bool {
        self.fg_color.is_none() && self.bg_color.is_none() && self.modifiers.is_empty()
    }

    /// Convert to render style
    pub fn to_render_style(&self) -> crate::render::style::Style {
        use crate::render::style::Style as RenderStyle;
        use crossterm::style::{Attributes, Colors};

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

    /// Interpolate this style toward another style.
    ///
    /// RGB colors are blended. Named and indexed terminal colors fall back to a
    /// discrete switch at the halfway point, matching terminal capability
    /// constraints.
    pub fn interpolate(self, target: Style, progress: f64) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        Self {
            fg_color: interpolate_optional_color(self.fg_color, target.fg_color, progress),
            bg_color: interpolate_optional_color(self.bg_color, target.bg_color, progress),
            modifiers: self.modifiers.interpolate(target.modifiers, progress),
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

impl Color {
    pub fn interpolate(self, target: Color, progress: f64) -> Color {
        let progress = progress.clamp(0.0, 1.0);
        match (self, target) {
            (Color::Rgb(sr, sg, sb), Color::Rgb(tr, tg, tb)) => Color::Rgb(
                lerp_u8(sr, tr, progress),
                lerp_u8(sg, tg, progress),
                lerp_u8(sb, tb, progress),
            ),
            (start, end) => {
                if progress < 0.5 {
                    start
                } else {
                    end
                }
            }
        }
    }
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

    pub fn interpolate(self, target: Self, progress: f64) -> Self {
        if progress < 0.5 {
            self
        } else {
            target
        }
    }
}

fn interpolate_optional_color(
    start: Option<Color>,
    end: Option<Color>,
    progress: f64,
) -> Option<Color> {
    match (start, end) {
        (Some(start), Some(end)) => Some(start.interpolate(end, progress)),
        (Some(start), None) => {
            if progress < 0.5 {
                Some(start)
            } else {
                None
            }
        }
        (None, Some(end)) => {
            if progress < 0.5 {
                None
            } else {
                Some(end)
            }
        }
        (None, None) => None,
    }
}

fn lerp_u8(start: u8, end: u8, progress: f64) -> u8 {
    (start as f64 + (end as f64 - start as f64) * progress)
        .round()
        .clamp(0.0, u8::MAX as f64) as u8
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_colors_interpolate_smoothly() {
        let start = Color::Rgb(0, 10, 20);
        let end = Color::Rgb(100, 110, 120);

        assert_eq!(start.interpolate(end, 0.5), Color::Rgb(50, 60, 70));
    }

    #[test]
    fn named_colors_switch_discretely() {
        assert_eq!(Color::Red.interpolate(Color::Blue, 0.49), Color::Red);
        assert_eq!(Color::Red.interpolate(Color::Blue, 0.5), Color::Blue);
    }

    #[test]
    fn style_interpolates_colors_and_modifiers() {
        let start = Style::default().fg(Color::Rgb(0, 0, 0));
        let end = Style::default().fg(Color::Rgb(10, 20, 30)).bold();

        let mid = start.interpolate(end, 0.5);
        assert_eq!(mid.fg_color, Some(Color::Rgb(5, 10, 15)));
        assert!(mid.modifiers.contains_bold());
    }
}
