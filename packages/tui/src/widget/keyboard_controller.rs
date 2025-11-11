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
}

impl<M> std::fmt::Debug for KeyboardController<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardController")
            .field("on_up", &self.on_up.is_some())
            .field("on_down", &self.on_down.is_some())
            .field("key_handlers", &self.key_handlers.len())
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

    /// Attach handlers for multiple keys at once
    ///
    /// This is a convenience method for registering multiple key handlers.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::KeyboardController;
    /// use tui::event::KeyCode;
    ///
    /// #[derive(Clone)]
    /// enum Message { A, B, C }
    ///
    /// let controller = KeyboardController::new()
    ///     .on_keys(&[
    ///         (KeyCode::Char('a'), Message::A),
    ///         (KeyCode::Char('b'), Message::B),
    ///         (KeyCode::Char('c'), Message::C),
    ///     ]);
    /// ```
    pub fn on_keys(mut self, mappings: &[(KeyCode, M)]) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        for (key_code, message) in mappings {
            let msg = message.clone();
            self.key_handlers.insert(*key_code, std::sync::Arc::new(move || msg.clone()));
        }
        self
    }

    /// Attach handlers for character keys using a closure
    pub fn on_chars<F>(mut self, chars: &[char], handler: F) -> Self
    where
        F: Fn(char) -> M + Send + Sync + 'static,
        M: 'static,
    {
        let handler = std::sync::Arc::new(handler);
        for &ch in chars {
            let handler_clone = handler.clone();
            self.key_handlers.insert(
                KeyCode::Char(ch),
                std::sync::Arc::new(move || handler_clone(ch)),
            );
        }
        self
    }

    /// Attach a handler for a character key (convenience method)
    ///
    /// Shorthand for `on_key(KeyCode::Char(c), handler)`.
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let controller = keyboard_controller()
    ///     .on('t', || Message::Tick)
    ///     .on('q', || Message::Quit);
    /// ```
    pub fn on<F>(self, ch: char, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_key(KeyCode::Char(ch), handler)
    }
}

impl<M: Send + Sync> Widget<M> for KeyboardController<M> {

    fn render(&self, _chunk: &mut render::chunk::Chunk) {
        // KeyboardController doesn't render anything - it's invisible
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        if let Event::Key(key_event) = event {
            // Check for Up/Down keys first
            match key_event.code {
                KeyCode::Up => {
                    if let Some(ref handler) = self.on_up {
                        let message = handler();
                        return EventResult::consumed_with(message);
                    }
                }
                KeyCode::Down => {
                    if let Some(ref handler) = self.on_down {
                        let message = handler();
                        return EventResult::consumed_with(message);
                    }
                }
                _ => {}
            }

            // Check for other registered key handlers
            if let Some(handler) = self.key_handlers.get(&key_event.code) {
                let message = handler();
                return EventResult::consumed_with(message);
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
}

/// Create a new keyboard controller (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// let controller = keyboard_controller()
///     .on('t', || Message::Tick)
///     .on('q', || Message::Quit);
/// ```
pub fn keyboard_controller<M>() -> KeyboardController<M> {
    KeyboardController::new()
}
