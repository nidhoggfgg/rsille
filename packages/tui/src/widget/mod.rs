//! Widget system - core trait and types

pub mod any;

// Widget implementations
pub mod button;
pub mod checkbox;
pub mod keyboard_controller;
pub mod label;
pub mod text_input;

// Incomplete widgets - not yet exported to public API
#[allow(dead_code)]
mod list;
#[allow(dead_code)]
mod progress_bar;

pub use any::AnyWidget;
pub use button::Button;
pub use checkbox::Checkbox;
pub use keyboard_controller::KeyboardController;
pub use label::Label;
pub use text_input::TextInput;
// Note: List and ProgressBar are not exported until fully implemented

// Re-export Area from render package
pub use render::area::Area;

use crate::event::{Event, EventResult};
use crate::layout::Constraints;

/// Core widget trait that all TUI widgets implement
pub trait Widget {
    /// The type of message this widget can produce
    type Message;

    /// Render the widget into the provided chunk at the specified area.
    ///
    /// # Arguments
    /// * `chunk` - Mutable chunk to draw into
    /// * `area` - Rectangular area allocated for this widget
    fn render(&self, chunk: &mut render::chunk::Chunk, area: Area);

    /// Handle an input event and return the result with any generated messages.
    ///
    /// # Arguments
    /// * `event` - The event to handle (keyboard, resize, etc.)
    ///
    /// # Returns
    /// * `EventResult::Consumed(messages)` if event was handled, with any produced messages
    /// * `EventResult::Ignored` if event should propagate
    fn handle_event(&mut self, event: &Event) -> EventResult<Self::Message>;

    /// Return the size constraints for this widget.
    ///
    /// Used by layout manager to calculate widget positions.
    fn constraints(&self) -> Constraints;

    /// Whether this widget can receive keyboard focus.
    ///
    /// # Returns
    /// * `true` for interactive widgets (Button, TextInput, etc.)
    /// * `false` for display-only widgets (Label, ProgressBar, etc.)
    fn focusable(&self) -> bool {
        false // default: not focusable
    }
}
