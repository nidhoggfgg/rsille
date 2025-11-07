//! Event system for handling user input
//!
//! This module re-exports event types from crossterm and provides
//! additional event handling utilities.

// Re-export crossterm event types
pub use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MediaKeyCode,
    ModifierKeyCode, MouseButton, MouseEvent, MouseEventKind,
};

// TUI-specific event handling modules
pub mod handler;
pub mod result;

pub use handler::{EventEmitter, EventHandler, EventHandlerWithContext};
pub use result::EventResult;
