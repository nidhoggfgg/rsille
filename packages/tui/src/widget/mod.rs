//! Widget system - core trait and types

mod into_widget;

// Widget implementations
mod button;
mod checkbox;
mod checkbox_group;
mod interactive;
mod keyboard_controller;
mod label;
mod list;
mod radio;
mod select;
mod text_input;

pub use button::{button, Button, ButtonVariant};
pub use checkbox::{checkbox, Checkbox};
pub use checkbox_group::{checkbox_group, CheckboxDirection, CheckboxGroup};
pub use interactive::{interactive, Interactive};
pub use into_widget::IntoWidget;
pub use keyboard_controller::{keyboard_controller, KeyboardController};
pub use label::{label, Label};
pub use list::{list, List, ListItem, SelectionEvent, SelectionMode};
pub use radio::{radio_group, RadioDirection, RadioGroup};
pub use select::{select, Select, SelectEvent, SelectItem};
pub use text_input::{text_input, TextInput, TextInputVariant};

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

    /// Returns whether this widget can receive keyboard focus.
    ///
    /// Focusable widgets can receive keyboard events when focused and will be
    /// included in the focus chain for Tab navigation.
    ///
    /// # Default
    /// Returns `false` - widgets are not focusable by default.
    fn focusable(&self) -> bool {
        false
    }

    /// Returns whether this widget currently has focus.
    ///
    /// This is used for rendering focus indicators (borders, highlights, etc.).
    ///
    /// # Default
    /// Returns `false` - no focus state by default.
    fn is_focused(&self) -> bool {
        false
    }

    /// Set the focus state of this widget.
    ///
    /// Called by the framework when focus changes. Widgets should store this state
    /// and use it for rendering focus indicators and handling keyboard events.
    ///
    /// # Arguments
    /// * `focused` - Whether the widget should be focused
    ///
    /// # Default
    /// Does nothing - widgets can override to track focus state.
    fn set_focused(&mut self, _focused: bool) {
        // Default: no-op
    }

    /// Build focus chain recursively (for containers)
    ///
    /// Allows containers to contribute their children's focus paths to the focus chain.
    /// Called during focus chain building to support nested containers.
    ///
    /// # Arguments
    /// * `current_path` - Current path in the widget tree
    /// * `chain` - Accumulated focus chain
    ///
    /// # Default
    /// Does nothing - only containers need to implement this.
    fn build_focus_chain_recursive(
        &self,
        _current_path: &mut Vec<usize>,
        _chain: &mut Vec<crate::focus::FocusPath>,
    ) {
        // Default: no-op (leaf widgets don't have children)
    }

    /// Update focus states recursively (for containers)
    ///
    /// Allows containers to update focus states of their children.
    /// Called during focus state updates to support nested containers.
    ///
    /// # Arguments
    /// * `current_path` - Current path in the widget tree
    /// * `focus_path` - The path of the focused widget (if any)
    ///
    /// # Default
    /// Does nothing - only containers need to implement this.
    fn update_focus_states_recursive(
        &mut self,
        _current_path: &[usize],
        _focus_path: Option<&crate::focus::FocusPath>,
    ) {
        // Default: no-op (leaf widgets don't have children)
    }
}
