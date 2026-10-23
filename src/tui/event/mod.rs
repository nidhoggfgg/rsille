//! Event system — re-exports from crossterm

pub use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MediaKeyCode,
    ModifierKeyCode, MouseButton, MouseEvent, MouseEventKind,
};

pub use crate::render::{InputCapabilities, KeyboardMode};

/// Ordered application input, observed before widget and shortcut handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    /// Emitted once after terminal setup, before the first terminal event.
    Capabilities(InputCapabilities),
    /// Includes all key kinds, focus changes, mouse, paste and resize events.
    Terminal(Event),
}

pub(crate) fn is_key_release(event: &Event) -> bool {
    matches!(event, Event::Key(key) if key.kind == KeyEventKind::Release)
}
