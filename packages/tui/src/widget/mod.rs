//! Widget system - core trait and types

mod into_widget;

mod keyboard_controller;
mod label;

pub use into_widget::IntoWidget;
pub use keyboard_controller::{keyboard_controller, KeyboardController};
pub use label::{label, Label};

use crate::event::{Event, EventResult};
use crate::layout::Constraints;

/// Core widget trait that all TUI widgets implement
///
/// This trait uses a generic parameter `M` for the message type, allowing widgets
/// to be stored as trait objects: `Box<dyn Widget<M>>`.
pub trait Widget<M>: Send + Sync {
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
    fn handle_event(&mut self, event: &Event) -> EventResult<M>;

    /// Return the size constraints for this widget.
    ///
    /// Used by layout manager to calculate widget positions.
    fn constraints(&self) -> Constraints;
}
