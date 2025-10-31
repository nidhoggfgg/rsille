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
    focused: bool,
    on_toggle: Option<EventHandler<M>>,
    pending_message: Option<M>,
}

impl<M> std::fmt::Debug for Checkbox<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("style", &self.style)
            .field("focused", &self.focused)
            .field("on_toggle", &self.on_toggle.is_some())
            .field("pending_message", &self.pending_message.is_some())
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
            focused: false,
            on_toggle: None,
            pending_message: None,
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
    /// - Keyboard: Press Space or Enter when the checkbox is focused
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

    /// Set focus state (managed by FocusManager)
    pub(crate) fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Take the pending message if any
    pub(crate) fn take_message(&mut self) -> Option<M> {
        self.pending_message.take()
    }

    /// Toggle the checked state
    fn toggle(&mut self) {
        self.checked = !self.checked;
    }
}

impl<M> Widget for Checkbox<M> {
    fn render(&self, buf: &mut Buffer, area: Rect) {
        if area.width < 4 || area.height == 0 {
            return;
        }

        // Render checkbox: [X] Label or [ ] Label
        let box_char = if self.checked { "[X]" } else { "[ ]" };
        let prefix = if self.focused { ">" } else { "" };

        let checkbox_text = if self.label.is_empty() {
            format!("{}{}", prefix, box_char)
        } else {
            format!("{}{} {}", prefix, box_char, self.label)
        };

        buf.set_string(area.x, area.y, &checkbox_text, self.style);
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::Key(key_event) if self.focused => {
                // Handle Space/Enter key as toggle when focused
                match key_event.code {
                    KeyCode::Space | KeyCode::Enter => {
                        self.toggle();
                        // Trigger toggle handler and store the message
                        if let Some(ref handler) = self.on_toggle {
                            self.pending_message = Some(handler());
                        }
                        return EventResult::Consumed;
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
                    // Trigger toggle handler and store the message
                    if let Some(ref handler) = self.on_toggle {
                        self.pending_message = Some(handler());
                    }
                    return EventResult::Consumed;
                }
            }
            _ => {}
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        // Checkbox width = 3 (for "[ ]") + 1 (space) + label width + 1 (focus indicator)
        let width =
            3 + if self.label.is_empty() {
                0
            } else {
                1 + self.label.width() as u16
            } + 1;

        let height = 1;

        Constraints {
            min_width: width,
            max_width: Some(width),
            min_height: height,
            max_height: Some(height),
            flex: None,
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}
