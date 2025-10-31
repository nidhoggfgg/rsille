//! Keyboard Controller widget for handling global keyboard events

use super::*;
use crate::event::handler::EventHandler;
use crate::event::{Event, KeyCode};
use crate::layout::Constraints;
use std::collections::HashMap;

/// Keyboard Controller widget for handling global keyboard events
///
/// This widget doesn't render anything but can intercept keyboard events
/// and emit messages in response to specific key presses.
///
/// # Examples
/// ```
/// use tui::widget::KeyboardController;
/// use tui::event::KeyCode;
///
/// #[derive(Clone)]
/// enum Message { Up, Down, Reset }
///
/// let controller = KeyboardController::new()
///     .on_up(|| Message::Up)
///     .on_down(|| Message::Down)
///     .on_key(KeyCode::Char('r'), || Message::Reset);
/// ```
#[derive(Clone)]
pub struct KeyboardController<M = ()> {
    on_up: Option<EventHandler<M>>,
    on_down: Option<EventHandler<M>>,
    key_handlers: std::collections::HashMap<KeyCode, EventHandler<M>>,
    pending_message: Option<M>,
}

impl<M> std::fmt::Debug for KeyboardController<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardController")
            .field("on_up", &self.on_up.is_some())
            .field("on_down", &self.on_down.is_some())
            .field("key_handlers", &self.key_handlers.len())
            .field("pending_message", &self.pending_message.is_some())
            .finish()
    }
}

impl<M> KeyboardController<M> {
    /// Create a new keyboard controller
    ///
    /// # Examples
    /// ```
    /// use tui::widget::KeyboardController;
    ///
    /// let controller = KeyboardController::<()>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            on_up: None,
            on_down: None,
            key_handlers: HashMap::new(),
            pending_message: None,
        }
    }

    /// Attach a handler for Up key presses
    ///
    /// # Examples
    /// ```
    /// use tui::widget::KeyboardController;
    ///
    /// #[derive(Clone)]
    /// enum Message { Increment }
    ///
    /// let controller = KeyboardController::new()
    ///     .on_up(|| Message::Increment);
    /// ```
    pub fn on_up<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_up = Some(std::sync::Arc::new(handler));
        self
    }

    /// Attach a handler for Down key presses
    ///
    /// # Examples
    /// ```
    /// use tui::widget::KeyboardController;
    ///
    /// #[derive(Clone)]
    /// enum Message { Decrement }
    ///
    /// let controller = KeyboardController::new()
    ///     .on_down(|| Message::Decrement);
    /// ```
    pub fn on_down<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_down = Some(std::sync::Arc::new(handler));
        self
    }

    /// Attach a handler for a specific key press
    ///
    /// # Examples
    /// ```
    /// use tui::widget::KeyboardController;
    /// use tui::event::KeyCode;
    ///
    /// #[derive(Clone)]
    /// enum Message { Reset }
    ///
    /// let controller = KeyboardController::new()
    ///     .on_key(KeyCode::Char('r'), || Message::Reset);
    /// ```
    pub fn on_key<F>(mut self, key_code: KeyCode, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.key_handlers.insert(key_code, std::sync::Arc::new(handler));
        self
    }

    /// Set focus state (managed by FocusManager)
    pub(crate) fn set_focused(&mut self, _focused: bool) {
        // KeyboardController doesn't have visual focus state
    }

    /// Take the pending message if any
    pub(crate) fn take_message(&mut self) -> Option<M> {
        self.pending_message.take()
    }
}

impl<M> Widget for KeyboardController<M> {
    fn render(&self, _buf: &mut Buffer, _area: Rect) {
        // KeyboardController doesn't render anything - it's invisible
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key_event) = event {
            // Check for Up/Down keys first
            match key_event.code {
                KeyCode::Up => {
                    if let Some(ref handler) = self.on_up {
                        self.pending_message = Some(handler());
                        return EventResult::Consumed;
                    }
                }
                KeyCode::Down => {
                    if let Some(ref handler) = self.on_down {
                        self.pending_message = Some(handler());
                        return EventResult::Consumed;
                    }
                }
                _ => {}
            }

            // Check for other registered key handlers
            if let Some(handler) = self.key_handlers.get(&key_event.code) {
                self.pending_message = Some(handler());
                return EventResult::Consumed;
            }
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        // KeyboardController takes no space
        Constraints {
            min_width: 0,
            max_width: Some(0),
            min_height: 0,
            max_height: Some(0),
            flex: None,
        }
    }

    fn focusable(&self) -> bool {
        // KeyboardController should be focusable to receive events
        true
    }
}
