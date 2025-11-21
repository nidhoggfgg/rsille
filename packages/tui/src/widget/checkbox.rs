//! Checkbox widget - interactive checkbox component

use super::*;
use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::style::{Style, ThemeManager};
use std::sync::Arc;

/// Interactive checkbox widget
///
/// A checkbox allows users to toggle a boolean state. Supports both keyboard and mouse interaction.
///
/// # Examples
/// ```
/// use tui::widget::Checkbox;
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     TermsAccepted(bool),
/// }
///
/// let checkbox = Checkbox::new("I accept the terms")
///     .checked(false)
///     .on_change(|checked| Message::TermsAccepted(checked));
/// ```
#[derive(Clone)]
pub struct Checkbox<M = ()> {
    label: String,
    checked: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    disabled: bool,
    focused: bool,
    on_change: Option<Arc<dyn Fn(bool) -> M + Send + Sync>>,
}

impl<M> std::fmt::Debug for Checkbox<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("custom_style", &self.custom_style)
            .field("custom_focus_style", &self.custom_focus_style)
            .field("disabled", &self.disabled)
            .field("focused", &self.focused)
            .field("on_change", &self.on_change.is_some())
            .finish()
    }
}

impl<M> Checkbox<M> {
    /// Create a new checkbox with the specified label
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Checkbox;
    ///
    /// let checkbox = Checkbox::<()>::new("Remember me");
    /// ```
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            custom_style: None,
            custom_focus_style: None,
            disabled: false,
            focused: false,
            on_change: None,
        }
    }

    /// Set the checked state
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Checkbox;
    ///
    /// let checkbox = Checkbox::<()>::new("Enabled")
    ///     .checked(true);
    /// ```
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set a custom change handler
    ///
    /// The handler will be called when the checkbox state changes via mouse click or keyboard activation.
    /// The handler receives the new checked state as a parameter.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Checkbox;
    ///
    /// #[derive(Clone)]
    /// enum Message { ToggleFeature(bool) }
    ///
    /// let checkbox = Checkbox::new("Enable feature")
    ///     .on_change(|checked| Message::ToggleFeature(checked));
    /// ```
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(bool) -> M + Send + Sync + 'static,
    {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Set the disabled state
    ///
    /// Disabled checkboxes cannot be interacted with and use muted styling.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Checkbox;
    ///
    /// let checkbox = Checkbox::<()>::new("Disabled option")
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
    /// use tui::widget::Checkbox;
    /// use tui::style::{Style, Color};
    ///
    /// let checkbox = Checkbox::<()>::new("Custom")
    ///     .style(Style::default().fg(Color::Cyan));
    /// ```
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Set a custom focus style (overrides theme focus styling)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Checkbox;
    /// use tui::style::{Style, Color};
    ///
    /// let checkbox = Checkbox::<()>::new("Custom Focus")
    ///     .focus_style(Style::default().fg(Color::Cyan).bold());
    /// ```
    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Get the effective style based on current state
    fn get_style(&self) -> Style {
        // Get base theme style based on state
        let base_style = ThemeManager::global().with_theme(|theme| {
            // Disabled state takes priority
            if self.disabled {
                return theme.styles.disabled;
            }

            // Focused state
            if self.focused {
                return theme.styles.interactive_focused;
            }

            // Normal state
            theme.styles.interactive
        });

        // If custom focus style is provided and checkbox is focused, use it
        if self.focused {
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

    /// Toggle the checked state and emit change event
    fn toggle(&mut self) -> Vec<M> {
        self.checked = !self.checked;
        if let Some(ref handler) = self.on_change {
            let message = handler(self.checked);
            vec![message]
        } else {
            vec![]
        }
    }
}

impl<M: Send + Sync> Widget<M> for Checkbox<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let style = self.get_style();
        let render_style = style.to_render_style();

        // Render checkbox symbol
        let checkbox_symbol = if self.checked { "[✓]" } else { "[ ]" };

        // Use success color (green) for checked checkbox to make it stand out
        let checkbox_style = if self.checked && !self.disabled {
            ThemeManager::global().with_theme(|theme| {
                Style::default()
                    .fg(theme.colors.success)
                    .bold()
                    .to_render_style()
            })
        } else {
            render_style
        };

        // Render checkbox symbol with highlight when checked
        let _ = chunk.set_string(0, 0, checkbox_symbol, checkbox_style);

        // Render label with a space after the checkbox
        // When focused, make the label text more prominent with color
        if !self.label.is_empty() {
            let label_x = 4; // Position after "[X] "
            let label_style = if self.focused && !self.disabled {
                // Use info color (blue/cyan) and bold when focused to make it stand out
                ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.info)
                        .bold()
                        .to_render_style()
                })
            } else {
                render_style
            };
            let _ = chunk.set_string(label_x, 0, &self.label, label_style);
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // Disabled checkboxes don't handle events
        if self.disabled {
            return EventResult::Ignored;
        }

        match event {
            // Keyboard events
            Event::Key(key_event) => {
                match key_event.code {
                    // Enter or Space toggles the checkbox
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        let messages = self.toggle();
                        return EventResult::Consumed(messages);
                    }
                    _ => {}
                }
            }

            // Mouse events
            Event::Mouse(mouse_event) => {
                match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let messages = self.toggle();
                        return EventResult::Consumed(messages);
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

        // Checkbox symbol "[X]" is 3 characters wide, plus 1 space, plus label width
        let checkbox_width = 3;
        let space_width = 1;
        let label_width = self.label.width() as u16;
        let total_width = checkbox_width + space_width + label_width;

        Constraints {
            min_width: total_width,
            max_width: Some(total_width),
            min_height: 1,
            max_height: Some(1),
            flex: None,
        }
    }

    fn focusable(&self) -> bool {
        // Checkboxes are focusable unless disabled
        !self.disabled
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

/// Create a new checkbox widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message { Toggled(bool) }
///
/// let cb = checkbox("Remember me")
///     .on_change(|checked| Message::Toggled(checked));
/// ```
pub fn checkbox<M>(label: impl Into<String>) -> Checkbox<M> {
    Checkbox::new(label)
}
