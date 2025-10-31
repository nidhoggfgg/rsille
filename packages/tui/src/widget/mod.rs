//! Widget system - core trait and types

pub mod any;
pub mod common;

// Widget implementations (will be added in US1)
pub mod button;
pub mod checkbox;
pub mod keyboard_controller;
pub mod label;
pub mod list;
pub mod progress_bar;
pub mod text_input;

pub use any::AnyWidget;
pub use button::Button;
pub use checkbox::Checkbox;
pub use common::{Position, Rect, Size};
pub use keyboard_controller::KeyboardController;
pub use label::Label;
pub use list::List;
pub use progress_bar::ProgressBar;
pub use text_input::TextInput;

use crate::buffer::Buffer;
use crate::event::{Event, EventResult};
use crate::layout::Constraints;

/// Core widget trait that all TUI widgets implement
pub trait Widget {
    /// Render the widget into the provided buffer at the specified area.
    ///
    /// # Arguments
    /// * `buf` - Mutable buffer to draw into
    /// * `area` - Rectangular area allocated for this widget
    fn render(&self, buf: &mut Buffer, area: Rect);

    /// Handle an input event and return the result.
    ///
    /// # Arguments
    /// * `event` - The event to handle (keyboard, resize, etc.)
    ///
    /// # Returns
    /// * `EventResult::Consumed` if event was handled
    /// * `EventResult::Ignored` if event should propagate
    fn handle_event(&mut self, event: &Event) -> EventResult;

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
