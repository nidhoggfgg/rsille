//! TextInput widget - user text entry

use super::*;
use crate::event::handler::EventHandler;
use crate::event::KeyCode;
use crate::style::Style;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// TextInput widget for user text entry
#[derive(Clone)]
pub struct TextInput<M = ()> {
    value: String,
    cursor: usize,
    style: Style,
    focused: bool,
    on_change: Option<EventHandler<M>>,
    pending_message: Option<M>,
}

impl<M> std::fmt::Debug for TextInput<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInput")
            .field("value", &self.value)
            .field("cursor", &self.cursor)
            .field("style", &self.style)
            .field("focused", &self.focused)
            .field("on_change", &self.on_change.is_some())
            .field("pending_message", &self.pending_message.is_some())
            .finish()
    }
}

impl<M> TextInput<M> {
    /// Create a new text input with initial value
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    ///
    /// let input: TextInput<()> = TextInput::new("");
    /// let prefilled: TextInput<()> = TextInput::new("Initial text");
    /// ```
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.len();
        Self {
            value,
            cursor,
            style: Style::default(),
            focused: false,
            on_change: None,
            pending_message: None,
        }
    }

    /// Set the input style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Attach a change handler that emits a message when the value changes
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    ///
    /// #[derive(Clone)]
    /// enum Message { TextChanged }
    ///
    /// let input = TextInput::new("").on_change(|| Message::TextChanged);
    /// ```
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_change = Some(std::sync::Arc::new(handler));
        self
    }

    /// Get the current value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set focus state (managed by FocusManager)
    pub(crate) fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Take the pending message if any
    pub(crate) fn take_message(&mut self) -> Option<M> {
        self.pending_message.take()
    }

    /// Move cursor to the left
    fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor to the right
    fn move_cursor_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start
    fn move_cursor_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    fn move_cursor_end(&mut self) {
        self.cursor = self.value.len();
    }

    /// Insert a character at the cursor position
    fn insert_char(&mut self, ch: char) {
        self.value.insert(self.cursor, ch);
        self.cursor += 1;
    }

    /// Delete character before cursor (backspace)
    fn delete_before(&mut self) {
        if self.cursor > 0 {
            self.value.remove(self.cursor - 1);
            self.cursor -= 1;
        }
    }

    /// Delete character at cursor (delete key)
    fn delete_at_cursor(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    /// Trigger the change handler if present
    #[allow(dead_code)] // Will be used in message-based event handling
    pub(crate) fn trigger_change(&self) -> Option<M> {
        self.on_change.as_ref().map(|handler| handler())
    }
}

impl<M> Widget for TextInput<M> {
    fn render(&self, buf: &mut Buffer, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Calculate display text with cursor
        let display_text = if self.focused {
            // Show cursor position
            let before = &self.value[..self.cursor];
            let after = &self.value[self.cursor..];

            if self.value.is_empty() {
                "_".to_string()
            } else {
                format!("{}|{}", before, after)
            }
        } else {
            self.value.clone()
        };

        // Truncate or pad to fit area
        let max_width = area.width as usize;
        let text_width = display_text.width();

        let final_text = if text_width > max_width {
            // Truncate if too long
            let mut truncated = String::new();
            let mut width = 0;
            for ch in display_text.chars() {
                let ch_width = ch.width().unwrap_or(1);
                if width + ch_width > max_width {
                    break;
                }
                truncated.push(ch);
                width += ch_width;
            }
            truncated
        } else {
            display_text
        };

        buf.set_string(area.x, area.y, &final_text, self.style);
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        // Only handle events when focused
        if !self.focused {
            return EventResult::Ignored;
        }

        if let Event::Key(key_event) = event {
            let mut changed = false;

            match key_event.code {
                KeyCode::Char(ch) => {
                    self.insert_char(ch);
                    changed = true;
                }
                KeyCode::Backspace => {
                    self.delete_before();
                    changed = true;
                }
                KeyCode::Delete => {
                    self.delete_at_cursor();
                    changed = true;
                }
                KeyCode::Left => {
                    self.move_cursor_left();
                }
                KeyCode::Right => {
                    self.move_cursor_right();
                }
                KeyCode::Home => {
                    self.move_cursor_home();
                }
                KeyCode::End => {
                    self.move_cursor_end();
                }
                _ => return EventResult::Ignored,
            }

            // Trigger change handler if content changed
            if changed {
                if let Some(ref handler) = self.on_change {
                    self.pending_message = Some(handler());
                }
            }

            return EventResult::Consumed;
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        // TextInput is flexible - grows to fill available space
        Constraints {
            min_width: 10, // Minimum 10 characters wide
            max_width: None,
            min_height: 1,
            max_height: Some(1),
            flex: Some(1.0), // Flexible width
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}
