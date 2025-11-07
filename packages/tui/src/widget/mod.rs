//! Widget system - core trait and types

mod any;

// Widget implementations
mod button;
mod checkbox;
mod keyboard_controller;
mod label;
mod spacer;
mod text_input;

#[allow(dead_code)]
mod progress_bar;

pub use any::AnyWidget;
pub use button::Button;
pub use checkbox::Checkbox;
pub use keyboard_controller::KeyboardController;
pub use label::Label;
pub use spacer::Spacer;
pub use text_input::TextInput;

use crate::event::{Event, EventResult};
use crate::layout::Constraints;

/// Core widget trait that all TUI widgets implement
pub trait Widget {
    /// The type of message this widget can produce
    type Message;

    /// Render the widget into the provided chunk.
    ///
    /// The widget should draw at relative coordinates (0, 0) within the chunk.
    /// The chunk contains its area which defines the widget's allocated space.
    ///
    /// # Arguments
    /// * `chunk` - Mutable chunk to draw into, containing the widget's allocated area
    fn render(&self, chunk: &mut render::chunk::Chunk);

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
}
