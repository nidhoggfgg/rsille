//! Colors, text attributes, borders, spacing, and themes for TUI widgets.
//!
//! Use [`Style`] for visual properties and [`Theme`] for shared semantic styles
//! and animation defaults. [`BorderStyle`] and [`Padding`] affect layout and are
//! configured on widgets separately from their style.
//!
//! Styles can be built with methods or parsed with [`Style::from_css`]. Use
//! [`Style::merge`] to fill unspecified values from a theme role.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::style::{Color, Style, Theme};
//!
//! let theme = Theme::dark();
//! let style = Style::default().fg(Color::Cyan).merge(theme.styles.text);
//! assert_eq!(style.fg_color, Some(Color::Cyan));
//! ```

mod border;
mod css;
mod error;
mod padding;
mod theme;

pub use border::{BorderChars, BorderStyle};
pub use padding::Padding;

pub use error::CssError;
pub use theme::{EffectSlot, Theme, ThemeBuilder, ThemeEffects, ThemeStyles};

/// Colors and text attributes for a widget.
///
/// Style contains only visual/themeable properties like colors and text modifiers.
/// Layout properties like border and padding should be set directly on widgets.
/// Unset colors and empty modifiers can be filled from another style with
/// [`merge`](Self::merge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    /// Foreground color, or `None` when unspecified.
    pub fg_color: Option<Color>,
    /// Background color, or `None` when unspecified.
    pub bg_color: Option<Color>,
    /// Text attributes; an empty set leaves attributes unspecified when merging.
    pub modifiers: TextModifiers,
}

impl Style {
    /// Creates an empty style with no colors or text attributes set.
    pub fn default() -> Self {
        Self {
            fg_color: None,
            bg_color: None,
            modifiers: TextModifiers::empty(),
        }
    }

    /// Sets the foreground color, replacing any previously specified color.
    pub fn fg(mut self, color: Color) -> Self {
        self.fg_color = Some(color);
        self
    }

    /// Sets the background color, replacing any previously specified color.
    pub fn bg(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self
    }

    /// Enables bold text while preserving the other attributes.
    pub fn bold(mut self) -> Self {
        self.modifiers = self.modifiers.with_bold();
        self
    }

    /// Enables italic text while preserving the other attributes.
    pub fn italic(mut self) -> Self {
        self.modifiers = self.modifiers.with_italic();
        self
    }

    /// Enables underlined text while preserving the other attributes.
    pub fn underlined(mut self) -> Self {
        self.modifiers = self.modifiers.with_underlined();
        self
    }

    /// Fills unspecified properties from another style.
    ///
    /// This is useful for combining explicit widget styles with theme defaults.
    /// Values from `self` take precedence over values from `other`.
    /// Colors are merged independently. Modifiers are taken as a whole: a
    /// nonempty modifier set in `self` replaces all modifiers from `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rsille::tui::style::{Style, Color};
    ///
    /// let theme_style = Style::default().fg(Color::White).bg(Color::Blue).bold();
    /// let widget_style = Style::default().fg(Color::Red).italic();
    /// let final_style = widget_style.merge(theme_style);
    /// assert_eq!(final_style.fg_color, Some(Color::Red));
    /// assert_eq!(final_style.bg_color, Some(Color::Blue));
    /// assert!(final_style.modifiers.contains_italic());
    /// assert!(!final_style.modifiers.contains_bold());
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

    /// Returns `true` when both colors are unset and no text attributes are enabled.
    pub fn is_empty(&self) -> bool {
        self.fg_color.is_none() && self.bg_color.is_none() && self.modifiers.is_empty()
    }

    /// Converts colors and supported attributes to the terminal renderer's style.
    ///
    /// Bold, italic, and underline are forwarded. Dim and reversed flags are
    /// currently omitted by this conversion. Unset colors remain unspecified.
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
    /// Progress is clamped to `0.0..=1.0`. Two RGB colors are blended channel by
    /// channel; all other color pairs, including an unset color, switch to the
    /// target at `0.5`. The entire modifier set also switches at `0.5`.
    pub fn interpolate(self, target: Style, progress: f64) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        Self {
            fg_color: interpolate_optional_color(self.fg_color, target.fg_color, progress),
            bg_color: interpolate_optional_color(self.bg_color, target.bg_color, progress),
            modifiers: self.modifiers.interpolate(target.modifiers, progress),
        }
    }
}

/// Named terminal colors, true-color RGB values, or 256-color palette entries.
///
/// Named and indexed colors depend on the terminal's palette. Red, green,
/// yellow, blue, magenta, and cyan map to crossterm's `Dark*` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// The terminal's black color.
    Black,
    /// The terminal's dark red color.
    Red,
    /// The terminal's dark green color.
    Green,
    /// The terminal's dark yellow color.
    Yellow,
    /// The terminal's dark blue color.
    Blue,
    /// The terminal's dark magenta color.
    Magenta,
    /// The terminal's dark cyan color.
    Cyan,
    /// The terminal's white color.
    White,
    /// True-color red, green, and blue channels, each in `0..=255`.
    Rgb(u8, u8, u8),
    /// An entry in the terminal's 256-color palette.
    Indexed(u8),
}

impl Color {
    /// Interpolates toward `target` with progress clamped to `0.0..=1.0`.
    ///
    /// RGB pairs use linear interpolation with each channel rounded to the
    /// nearest integer. Other pairs return `self` below `0.5` and `target`
    /// otherwise, including pairs with only one RGB color.
    ///
    /// # Examples
    ///
    /// ```
    /// use rsille::tui::style::Color;
    ///
    /// assert_eq!(
    ///     Color::Rgb(0, 10, 20).interpolate(Color::Rgb(100, 110, 120), 0.5),
    ///     Color::Rgb(50, 60, 70),
    /// );
    /// assert_eq!(Color::Red.interpolate(Color::Blue, 0.5), Color::Blue);
    /// ```
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

/// A set of text attribute flags.
///
/// Start with [`empty`](Self::empty) and combine attributes with the `with_*`
/// methods. [`Style::to_render_style`] currently forwards only bold, italic,
/// and underline; dim and reversed flags are stored but not rendered there.
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

    /// Creates a set with no enabled attributes.
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Enables bold while preserving existing flags.
    pub const fn with_bold(mut self) -> Self {
        self.bits |= Self::BOLD;
        self
    }

    /// Enables dim while preserving existing flags.
    pub const fn with_dim(mut self) -> Self {
        self.bits |= Self::DIM;
        self
    }

    /// Enables italic while preserving existing flags.
    pub const fn with_italic(mut self) -> Self {
        self.bits |= Self::ITALIC;
        self
    }

    /// Enables underline while preserving existing flags.
    pub const fn with_underlined(mut self) -> Self {
        self.bits |= Self::UNDERLINED;
        self
    }

    /// Enables reversed foreground and background while preserving existing flags.
    pub const fn with_reversed(mut self) -> Self {
        self.bits |= Self::REVERSED;
        self
    }

    /// Returns whether bold is enabled.
    pub const fn contains_bold(&self) -> bool {
        self.bits & Self::BOLD != 0
    }

    /// Returns whether italic is enabled.
    pub const fn contains_italic(&self) -> bool {
        self.bits & Self::ITALIC != 0
    }

    /// Returns whether underline is enabled.
    pub const fn contains_underlined(&self) -> bool {
        self.bits & Self::UNDERLINED != 0
    }

    /// Returns whether no attributes are enabled.
    pub const fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// Switches the entire set from `self` to `target` at progress `0.5`.
    ///
    /// Returns `self` for values below `0.5` and `target` otherwise; flags are
    /// not blended or combined.
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

/// Allows TUI colors to be used by Canvas sample and background painting APIs.
impl From<Color> for crossterm::style::Color {
    fn from(color: Color) -> Self {
        color_to_crossterm(color)
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
