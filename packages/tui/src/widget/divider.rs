//! Divider widget - visual separator line

use super::*;
use crate::style::{BorderStyle, Style, ThemeManager};

/// Direction of the divider line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerDirection {
    /// Horizontal line (─────)
    Horizontal,
    /// Vertical line (│)
    Vertical,
}

/// Text position on the divider (for horizontal dividers with text)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerTextPosition {
    /// Text aligned to the left
    Left,
    /// Text centered (default)
    #[default]
    Center,
    /// Text aligned to the right
    Right,
}

/// Divider visual variants
///
/// Different visual styles for dividers based on their semantic purpose and visual weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerVariant {
    /// Solid line (default) - ─────
    #[default]
    Solid,
    /// Dashed line - ╌╌╌╌╌
    Dashed,
    /// Dotted line - ·····
    Dotted,
    /// Heavy/thick line - ━━━━━
    Heavy,
    /// Double line - ═════
    Double,
    /// Faded/subtle line - ┈┈┈┈┈
    Faded,
}

impl DividerVariant {
    /// Get the horizontal and vertical characters for this variant
    fn chars(&self) -> (char, char) {
        match self {
            DividerVariant::Solid => ('─', '│'),
            DividerVariant::Dashed => ('╌', '╎'),
            DividerVariant::Dotted => ('·', '┊'),
            DividerVariant::Heavy => ('━', '┃'),
            DividerVariant::Double => ('═', '║'),
            DividerVariant::Faded => ('┈', '┊'),
        }
    }
}

/// Divider widget for creating visual separators
///
/// # Examples
/// ```no_run
/// use tui::widget::divider;
/// use tui::widget::{DividerDirection, DividerVariant};
///
/// // Horizontal divider
/// let h_divider = divider()
///     .horizontal();
///
/// // Vertical divider with custom variant
/// let v_divider = divider()
///     .vertical()
///     .variant(DividerVariant::Heavy);
///
/// // Dashed divider
/// let dashed = divider()
///     .horizontal()
///     .variant(DividerVariant::Dashed);
///
/// // Divider with text
/// let with_text = divider()
///     .horizontal()
///     .text("Section Title");
/// ```
#[derive(Debug, Clone)]
pub struct Divider<M = ()> {
    direction: DividerDirection,
    variant: DividerVariant,
    style: Style,
    text: Option<String>,
    text_position: DividerTextPosition,
    text_spacing: u16,
    constraints: Constraints,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> Divider<M> {
    /// Create a new horizontal divider
    pub fn new() -> Self {
        Self {
            direction: DividerDirection::Horizontal,
            variant: DividerVariant::default(),
            style: Style::default(),
            text: None,
            text_position: DividerTextPosition::default(),
            text_spacing: 1,
            constraints: Constraints::content(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the divider direction to horizontal
    pub fn horizontal(mut self) -> Self {
        self.direction = DividerDirection::Horizontal;
        self
    }

    /// Set the divider direction to vertical
    pub fn vertical(mut self) -> Self {
        self.direction = DividerDirection::Vertical;
        self
    }

    /// Set the divider variant
    ///
    /// # Arguments
    /// * `variant` - Divider variant (Solid, Dashed, Dotted, Heavy, Double, Faded)
    ///
    /// # Examples
    /// ```no_run
    /// use tui::widget::{divider, DividerVariant};
    ///
    /// let divider = divider().variant(DividerVariant::Dashed);
    /// ```
    pub fn variant(mut self, variant: DividerVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set text to display on the divider (only for horizontal dividers)
    ///
    /// # Arguments
    /// * `text` - Text to display on the divider
    ///
    /// # Examples
    /// ```no_run
    /// use tui::widget::divider;
    ///
    /// let divider = divider().text("Section Title");
    /// ```
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set the text position on the divider
    ///
    /// # Arguments
    /// * `position` - Text position (Left, Center, Right)
    ///
    /// # Examples
    /// ```no_run
    /// use tui::widget::{divider, DividerTextPosition};
    ///
    /// let divider = divider()
    ///     .text("Title")
    ///     .text_position(DividerTextPosition::Left);
    /// ```
    pub fn text_position(mut self, position: DividerTextPosition) -> Self {
        self.text_position = position;
        self
    }

    /// Set the spacing around text (in characters)
    ///
    /// # Arguments
    /// * `spacing` - Number of space characters on each side of the text
    ///
    /// # Examples
    /// ```no_run
    /// use tui::widget::divider;
    ///
    /// let divider = divider()
    ///     .text("Title")
    ///     .text_spacing(2); // "── Title ──"
    /// ```
    pub fn text_spacing(mut self, spacing: u16) -> Self {
        self.text_spacing = spacing;
        self
    }

    /// Set the border style for the divider line (deprecated, use variant instead)
    ///
    /// # Arguments
    /// * `style` - Border style (Single, Double, Rounded, Thick)
    #[deprecated(since = "0.1.0", note = "Use variant() instead")]
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        // Map BorderStyle to DividerVariant for backward compatibility
        self.variant = match style {
            BorderStyle::None => DividerVariant::Faded,
            BorderStyle::Single => DividerVariant::Solid,
            BorderStyle::Double => DividerVariant::Double,
            BorderStyle::Rounded => DividerVariant::Solid,
            BorderStyle::Thick => DividerVariant::Heavy,
        };
        self
    }

    /// Set the divider style (color, modifiers)
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set fixed width (for horizontal dividers)
    pub fn width(mut self, width: u16) -> Self {
        self.constraints.min_width = width;
        self.constraints.max_width = Some(width);
        self
    }

    /// Set fixed height (for vertical dividers)
    pub fn height(mut self, height: u16) -> Self {
        self.constraints.min_height = height;
        self.constraints.max_height = Some(height);
        self
    }

    /// Make the divider flexible with the given flex factor
    pub fn flex(mut self, flex: f32) -> Self {
        self.constraints.flex = Some(flex);
        self
    }

    /// Fill all available space
    pub fn fill(mut self) -> Self {
        self.constraints = Constraints::fill();
        self
    }
}

impl<M> Default for Divider<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Send + Sync> Widget<M> for Divider<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        // Apply theme: merge explicit style with theme border color
        let theme_style =
            ThemeManager::global().with_theme(|theme| Style::default().fg(theme.colors.border));
        let final_style = self.style.merge(theme_style);

        // Convert TUI style to render style
        let render_style = final_style.to_render_style();

        // Get the characters for the current variant and direction
        let (h_char, v_char) = self.variant.chars();

        match self.direction {
            DividerDirection::Horizontal => {
                // Check if we have text to display
                if let Some(ref text) = self.text {
                    self.render_horizontal_with_text(chunk, h_char, text, render_style);
                } else {
                    // Draw horizontal line without text
                    let line = h_char.to_string().repeat(area.width() as usize);
                    let _ = chunk.set_string(0, 0, &line, render_style);
                }
            }
            DividerDirection::Vertical => {
                // Draw vertical line using the vertical character
                for y in 0..area.height() {
                    let _ = chunk.set_string(0, y, &v_char.to_string(), render_style);
                }
            }
        }
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult<M> {
        // Divider doesn't handle events
        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        // Use explicit constraints if set, otherwise use default based on direction
        if self.constraints.max_width.is_some()
            || self.constraints.max_height.is_some()
            || self.constraints.flex.is_some()
        {
            return self.constraints;
        }

        match self.direction {
            DividerDirection::Horizontal => {
                // Horizontal divider: fill width, 1 row height
                Constraints {
                    min_width: 1,
                    max_width: None, // Will expand to fill
                    min_height: 1,
                    max_height: Some(1),
                    flex: Some(1.0), // Fill horizontal space by default
                }
            }
            DividerDirection::Vertical => {
                // Vertical divider: 1 column width, fill height
                Constraints {
                    min_width: 1,
                    max_width: Some(1),
                    min_height: 1,
                    max_height: None, // Will expand to fill
                    flex: None,
                }
            }
        }
    }
}

impl<M> Divider<M> {
    /// Render horizontal divider with text
    fn render_horizontal_with_text(
        &self,
        chunk: &mut render::chunk::Chunk,
        line_char: char,
        text: &str,
        render_style: render::style::Style,
    ) {
        use unicode_width::UnicodeWidthStr;

        let area = chunk.area();
        let width = area.width() as usize;

        // Calculate text width including spacing
        let text_width = text.width();
        let spacing = self.text_spacing as usize;
        let total_text_width = text_width + spacing * 2;

        // If text is too wide, just render the line
        if total_text_width >= width {
            let line = line_char.to_string().repeat(width);
            let _ = chunk.set_string(0, 0, &line, render_style);
            return;
        }

        // Calculate line lengths based on text position
        let (left_line_len, right_line_len) = match self.text_position {
            DividerTextPosition::Left => {
                // "─ Text ───────────"
                let right_len = width.saturating_sub(total_text_width + spacing);
                (spacing, right_len)
            }
            DividerTextPosition::Center => {
                // "────── Text ──────"
                let remaining = width.saturating_sub(total_text_width);
                let left_len = remaining / 2;
                let right_len = remaining - left_len;
                (left_len, right_len)
            }
            DividerTextPosition::Right => {
                // "───────────── Text ─"
                let left_len = width.saturating_sub(total_text_width + spacing);
                (left_len, spacing)
            }
        };

        // Build the complete line
        let mut result = String::new();

        // Left line
        result.push_str(&line_char.to_string().repeat(left_line_len));

        // Spacing before text
        result.push_str(&" ".repeat(spacing));

        // Text
        result.push_str(text);

        // Spacing after text
        result.push_str(&" ".repeat(spacing));

        // Right line
        result.push_str(&line_char.to_string().repeat(right_line_len));

        // Render the line
        let _ = chunk.set_string(0, 0, &result, render_style);
    }
}

/// Create a new divider widget (convenience function)
///
/// # Examples
/// ```no_run
/// use tui::prelude::*;
///
/// // Horizontal divider
/// let h = divider().horizontal();
///
/// // Vertical divider
/// let v = divider().vertical();
///
/// // Custom variant divider
/// let custom = divider()
///     .horizontal()
///     .variant(DividerVariant::Dashed)
///     .width(20);
///
/// // Heavy vertical divider
/// let heavy = divider()
///     .vertical()
///     .variant(DividerVariant::Heavy);
/// ```
pub fn divider<M>() -> Divider<M> {
    Divider::new()
}
