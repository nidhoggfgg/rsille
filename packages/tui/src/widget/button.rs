//! Button widget - interactive button component

use super::*;
use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::style::{Color, Style, ThemeManager};
use std::sync::Arc;

/// Button style variants
///
/// Different visual styles for buttons based on their semantic purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Primary action button (solid background with primary color)
    Primary,
    /// Secondary action button (solid background with secondary color)
    Secondary,
    /// Ghost button (transparent background with border)
    Ghost,
    /// Link-style button (text only, no background or border)
    Link,
    /// Destructive action button (danger color for delete/remove operations)
    Destructive,
}

impl Default for ButtonVariant {
    fn default() -> Self {
        ButtonVariant::Primary
    }
}

/// Internal button state for interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonState {
    /// Normal state
    Normal,
    /// Mouse hover state
    Hover,
    /// Keyboard focus state
    Focused,
}

impl Default for ButtonState {
    fn default() -> Self {
        ButtonState::Normal
    }
}

/// Interactive button widget
///
/// Supports multiple visual variants, custom styling, and both keyboard and mouse interaction.
///
/// # Examples
/// ```
/// use tui::widget::{Button, ButtonVariant};
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     Submit,
///     Cancel,
/// }
///
/// let submit_btn = Button::new("Submit")
///     .variant(ButtonVariant::Primary)
///     .on_click(|| Message::Submit);
///
/// let cancel_btn = Button::new("Cancel")
///     .variant(ButtonVariant::Secondary)
///     .on_click(|| Message::Cancel);
/// ```
#[derive(Clone)]
pub struct Button<M = ()> {
    label: String,
    variant: ButtonVariant,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    disabled: bool,
    state: ButtonState,
    on_click: Option<Arc<dyn Fn() -> M + Send + Sync>>,
}

impl<M> std::fmt::Debug for Button<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("label", &self.label)
            .field("variant", &self.variant)
            .field("custom_style", &self.custom_style)
            .field("custom_focus_style", &self.custom_focus_style)
            .field("disabled", &self.disabled)
            .field("state", &self.state)
            .field("on_click", &self.on_click.is_some())
            .finish()
    }
}

impl<M> Button<M> {
    /// Create a new button with the specified label
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Button;
    ///
    /// let button = Button::<()>::new("Click me");
    /// ```
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::default(),
            custom_style: None,
            custom_focus_style: None,
            disabled: false,
            state: ButtonState::default(),
            on_click: None,
        }
    }

    /// Set the button variant
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{Button, ButtonVariant};
    ///
    /// let button = Button::<()>::new("Delete")
    ///     .variant(ButtonVariant::Destructive);
    /// ```
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set a custom click handler
    ///
    /// The handler will be called when the button is clicked via mouse or activated via keyboard.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Button;
    ///
    /// #[derive(Clone)]
    /// enum Message { Clicked }
    ///
    /// let button = Button::new("Click me")
    ///     .on_click(|| Message::Clicked);
    /// ```
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_click = Some(Arc::new(handler));
        self
    }

    /// Set the disabled state
    ///
    /// Disabled buttons cannot be interacted with and use muted styling.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Button;
    ///
    /// let button = Button::<()>::new("Disabled")
    ///     .disabled(true);
    /// ```
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set a custom style (overrides theme styling)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Button;
    /// use tui::style::{Style, Color};
    ///
    /// let button = Button::<()>::new("Custom")
    ///     .style(Style::default().fg(Color::Cyan).bg(Color::Black));
    /// ```
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Set a custom focus style (overrides theme focus styling)
    ///
    /// This allows advanced customization of the button's appearance when focused.
    /// For most use cases, the theme's default focus styling is sufficient.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Button;
    /// use tui::style::{Style, Color};
    ///
    /// let button = Button::<()>::new("Custom Focus")
    ///     .focus_style(Style::default().fg(Color::Cyan).bold());
    /// ```
    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Get the effective style based on current state and variant
    fn get_style(&self) -> Style {
        // Get base theme style based on state and variant
        let base_style = ThemeManager::global().with_theme(|theme| {
            // Disabled state takes priority
            if self.disabled {
                return theme.styles.disabled;
            }

            // Match variant and state
            match self.variant {
                ButtonVariant::Primary => match self.state {
                    ButtonState::Focused => theme.styles.primary_action_focused,
                    ButtonState::Hover => theme.styles.primary_action_hover,
                    ButtonState::Normal => theme.styles.primary_action,
                },
                ButtonVariant::Secondary => match self.state {
                    ButtonState::Focused => theme.styles.secondary_action_focused,
                    ButtonState::Hover => theme.styles.secondary_action_hover,
                    ButtonState::Normal => theme.styles.secondary_action,
                },
                ButtonVariant::Ghost => match self.state {
                    ButtonState::Focused => theme.styles.interactive_focused,
                    ButtonState::Hover => theme.styles.hover,
                    ButtonState::Normal => Style::default().fg(theme.colors.text),
                },
                ButtonVariant::Link => match self.state {
                    ButtonState::Focused => theme.styles.interactive_focused.underlined(),
                    ButtonState::Hover => theme.styles.hover.underlined(),
                    ButtonState::Normal => theme.styles.text.underlined(),
                },
                ButtonVariant::Destructive => match self.state {
                    ButtonState::Focused => Style::default()
                        .fg(Color::White)
                        .bg(theme.colors.danger)
                        .bold(),
                    ButtonState::Hover => Style::default()
                        .fg(Color::White)
                        .bg(theme.colors.danger)
                        .bold(),
                    ButtonState::Normal => {
                        Style::default().fg(Color::White).bg(theme.colors.danger)
                    }
                },
            }
        });

        // If custom focus style is provided and button is focused, use it
        if self.state == ButtonState::Focused {
            if let Some(ref focus_style) = self.custom_focus_style {
                return focus_style.merge(base_style);
            }
        }

        // Merge custom style if provided (custom style takes precedence)
        self.custom_style
            .as_ref()
            .map(|s| s.merge(base_style))
            .unwrap_or(base_style)
    }
}

impl<M: Send + Sync> Widget<M> for Button<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let style = self.get_style();
        let render_style = style.to_render_style();

        let width = area.width();
        let height = area.height();
        let is_focused = self.state == ButtonState::Focused;

        // Render background for solid variants
        if matches!(
            self.variant,
            ButtonVariant::Primary | ButtonVariant::Secondary | ButtonVariant::Destructive
        ) {
            let _ = chunk.fill(0, 0, width, height, ' ', render_style);
        }

        // Render border for Ghost variant
        if matches!(self.variant, ButtonVariant::Ghost) {
            // Get border characters and color from theme
            use crate::style::BorderStyle;
            let border_chars = BorderStyle::Single.chars();

            // Use focus_ring color when focused, otherwise use regular border color
            let border_style = ThemeManager::global().with_theme(|theme| {
                if is_focused {
                    Style::default()
                        .fg(theme.colors.focus_ring)
                        .to_render_style()
                } else {
                    Style::default().fg(theme.colors.border).to_render_style()
                }
            });

            // Top and bottom borders
            if width >= 2 {
                let _ = chunk.set_char(0, 0, border_chars.top_left, border_style);
                let _ = chunk.set_char(width - 1, 0, border_chars.top_right, border_style);
                for x in 1..width - 1 {
                    let _ = chunk.set_char(x, 0, border_chars.horizontal, border_style);
                }

                if height > 0 {
                    let _ = chunk.set_char(0, height - 1, border_chars.bottom_left, border_style);
                    let _ = chunk.set_char(
                        width - 1,
                        height - 1,
                        border_chars.bottom_right,
                        border_style,
                    );
                    for x in 1..width - 1 {
                        let _ =
                            chunk.set_char(x, height - 1, border_chars.horizontal, border_style);
                    }
                }
            }

            // Left and right borders
            if height > 2 {
                for y in 1..height - 1 {
                    let _ = chunk.set_char(0, y, border_chars.vertical, border_style);
                    let _ = chunk.set_char(width - 1, y, border_chars.vertical, border_style);
                }
            }
        }

        // Calculate text position (centered)
        use unicode_width::UnicodeWidthStr;
        let text_width = self.label.width() as u16;

        // For Ghost variant, account for border
        let padding_offset = if matches!(self.variant, ButtonVariant::Ghost) {
            1
        } else {
            0
        };

        let available_width = width.saturating_sub(padding_offset * 2);
        let text_x = if available_width > text_width {
            padding_offset + (available_width - text_width) / 2
        } else {
            padding_offset
        };

        let text_y = height / 2;

        // Render label text
        // For focused state, the style already includes bold from theme
        let _ = chunk.set_string(text_x, text_y, &self.label, render_style);
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // Disabled buttons don't handle events
        if self.disabled {
            return EventResult::Ignored;
        }

        match event {
            // Keyboard events
            Event::Key(key_event) => {
                match key_event.code {
                    // Enter or Space activates the button
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(ref handler) = self.on_click {
                            let message = handler();
                            return EventResult::consumed_with(message);
                        }
                    }
                    _ => {}
                }
            }

            // Mouse events (Note: actual mouse position handling needs area context)
            Event::Mouse(mouse_event) => {
                // We don't have access to the chunk here, so we'll handle basic mouse events
                // In a real implementation, the container would need to provide area information
                match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some(ref handler) = self.on_click {
                            let message = handler();
                            return EventResult::consumed_with(message);
                        }
                    }
                    MouseEventKind::Moved => {
                        // Update hover state
                        // Note: This is simplified - proper implementation needs area bounds checking
                        self.state = ButtonState::Hover;
                    }
                    _ => {}
                }
            }

            _ => {}
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        use unicode_width::UnicodeWidthStr;
        let label_width = self.label.width() as u16;

        // Calculate total width and height based on variant
        // Add 2 extra chars for focus indicators (> <)
        let (total_width, height) = match self.variant {
            // Ghost has border (1 char on each side) + padding (2 chars on each side)
            // and needs 3 rows (top border + text + bottom border)
            ButtonVariant::Ghost => (label_width + 6, 3),
            // Other variants: padding (2 chars on each side) + focus indicators (2 chars)
            _ => (label_width + 4, 1),
        };

        Constraints {
            min_width: total_width,
            max_width: Some(total_width),
            min_height: height,
            max_height: Some(height),
            flex: None,
        }
    }

    fn focusable(&self) -> bool {
        // Buttons are focusable unless disabled
        !self.disabled
    }

    fn is_focused(&self) -> bool {
        // Check if current state is focused
        self.state == ButtonState::Focused
    }

    fn set_focused(&mut self, focused: bool) {
        // Update state based on focus
        if focused {
            self.state = ButtonState::Focused;
        } else if self.state == ButtonState::Focused {
            // Only reset to Normal if we were focused
            // (preserve Hover state if mouse is over button)
            self.state = ButtonState::Normal;
        }
    }
}

/// Create a new button widget (convenience function)
///
/// Creates a button with Primary variant by default.
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message { Submit }
///
/// let btn = button("Submit")
///     .on_click(|| Message::Submit);
/// ```
pub fn button<M>(label: impl Into<String>) -> Button<M> {
    Button::new(label)
}
