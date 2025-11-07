//! Checkbox widget - boolean toggle

use super::*;
use crate::event::handler::EventHandler;
use crate::event::KeyCode;
use crate::style::Style;
use unicode_width::UnicodeWidthStr;

/// Checkbox widget for boolean values
#[derive(Clone)]
pub struct Checkbox<M = ()> {
    label: String,
    checked: bool,
    style: Style,
    on_toggle: Option<EventHandler<M>>,
}

impl<M> std::fmt::Debug for Checkbox<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("style", &self.style)
            .field("on_toggle", &self.on_toggle.is_some())
            .finish()
    }
}

impl<M> Checkbox<M> {
    /// Create a new checkbox with label and initial state
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Checkbox;
    ///
    /// let checkbox: Checkbox<()> = Checkbox::new("Accept terms", false);
    /// let checked: Checkbox<()> = Checkbox::new("Enabled", true);
    /// ```
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        Self {
            label: label.into(),
            checked,
            style: Style::default(),
            on_toggle: None,
        }
    }

    /// Set the checkbox style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Attach a toggle handler that emits a message when toggled
    ///
    /// The checkbox can be toggled by:
    /// - Keyboard: Press Space or Enter
    /// - Mouse: Click the checkbox with the left mouse button
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Checkbox;
    ///
    /// #[derive(Clone)]
    /// enum Message { ToggleAccept }
    ///
    /// let checkbox = Checkbox::new("Accept", false)
    ///     .on_toggle(|| Message::ToggleAccept);
    /// ```
    pub fn on_toggle<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_toggle = Some(std::sync::Arc::new(handler));
        self
    }

    /// Get the checked state
    pub fn checked(&self) -> bool {
        self.checked
    }

    /// Get the label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Toggle the checked state
    fn toggle(&mut self) {
        self.checked = !self.checked;
    }
}

impl<M> Widget for Checkbox<M> {
    type Message = M;

    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() < 4 || area.height() == 0 {
            return;
        }

        // Render checkbox: [X] Label or [ ] Label
        let box_char = if self.checked { "[X]" } else { "[ ]" };

        let checkbox_text = if self.label.is_empty() {
            format!("{}", box_char)
        } else {
            format!("{} {}", box_char, self.label)
        };

        // Convert TUI style to render style
        let render_style = self.style.to_render_style();

        let _ = chunk.set_string(0, 0, &checkbox_text, render_style);
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        match event {
            Event::Key(key_event) => {
                // Handle Space/Enter key as toggle
                match key_event.code {
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        self.toggle();
                        // Trigger toggle handler and return the message
                        if let Some(ref handler) = self.on_toggle {
                            let message = handler();
                            return EventResult::consumed_with(message);
                        }
                        return EventResult::consumed();
                    }
                    _ => {}
                }
            }
            Event::Mouse(mouse_event) => {
                // Handle mouse clicks (left button)
                if let crate::event::MouseEventKind::Up(crate::event::MouseButton::Left) =
                    mouse_event.kind
                {
                    self.toggle();
                    // Trigger toggle handler and return the message
                    if let Some(ref handler) = self.on_toggle {
                        let message = handler();
                        return EventResult::consumed_with(message);
                    }
                    return EventResult::consumed();
                }
            }
            _ => {}
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        // Checkbox width = 3 (for "[ ]") + 1 (space) + label width
        let width =
            3 + if self.label.is_empty() {
                0
            } else {
                1 + self.label.width() as u16
            };

        let height = 1;

        Constraints {
            min_width: width,
            max_width: Some(width),
            min_height: height,
            max_height: Some(height),
            flex: None,
        }
    }
}
