//! Widget system - core trait and types

mod into_widget;

// Common widget utilities
pub mod common;

// Widget implementations
mod button;
mod checkbox;
mod checkbox_group;
mod code_block;
mod dialog;
mod divider;
mod label;
mod list;
mod progress_bar;
mod radio;
mod select;
mod spacer;
mod table;
mod text_input;
mod textarea;
mod wrapper;

pub use button::{button, Button, ButtonVariant};
pub use checkbox::{checkbox, Checkbox};
pub use checkbox_group::{checkbox_group, CheckboxDirection, CheckboxGroup};
pub use code_block::{code_block, CodeBlock, LineMarker};
pub use dialog::{alert, confirm, dialog, Dialog, DialogMessage, DialogSize};
pub use divider::{divider, Divider, DividerDirection, DividerTextPosition, DividerVariant};
pub use into_widget::IntoWidget;
pub use label::{label, Label};
pub use list::{list, List, ListItem, SelectionEvent, SelectionMode};
pub use progress_bar::{progress_bar, LabelPosition, ProgressBar, ProgressMode};
pub use radio::{radio_group, RadioDirection, RadioGroup};
pub use select::{select, Select, SelectEvent, SelectItem};
pub use spacer::{spacer, Spacer};
pub use table::{table, Column, ColumnWidth, Table, TableSelectionEvent, TableVariant};
pub use text_input::{text_input, TextInput, TextInputVariant};
pub use textarea::{textarea, Selection, Textarea, TextareaVariant};
pub use wrapper::{enhanced, Enhanced};

use crate::event::{Event, EventResult};
use crate::layout::Constraints;

/// Simplified widget trait for leaf components that only need rendering
///
/// SimpleWidget provides a pattern for implementing simple widgets with minimal boilerplate.
/// While it doesn't automatically implement Widget, it serves as a guide for what methods
/// are truly essential for non-interactive widgets.
///
/// # When to use SimpleWidget
///
/// SimpleWidget is useful as a pattern for:
/// - Pure display components (Label, Divider, Spacer)
/// - Components with no user interaction
/// - Custom widgets that don't need event handling
///
/// # Usage Pattern
///
/// 1. Implement SimpleWidget for your type
/// 2. Either:
///    - Manually implement the full Widget trait (copy from the example below)
///    - Or wrap your widget with `enhanced()` to add interactivity
///
/// # Examples
///
/// ## Pattern 1: Manual Widget implementation
///
/// ```
/// use tui::widget::{SimpleWidget, Widget};
/// use tui::event::{Event, EventResult};
/// use tui::layout::Constraints;
/// use render::chunk::Chunk;
///
/// struct MyWidget {
///     text: String,
/// }
///
/// impl<M: Send + Sync> SimpleWidget<M> for MyWidget {
///     fn render(&self, chunk: &mut Chunk) {
///         let _ = chunk.set_string(0, 0, &self.text, Default::default());
///     }
///
///     fn constraints(&self) -> Constraints {
///         Constraints {
///             min_width: self.text.len() as u16,
///             max_width: Some(self.text.len() as u16),
///             min_height: 1,
///             max_height: Some(1),
///             flex: None,
///         }
///     }
/// }
///
/// // Manual Widget implementation (boilerplate)
/// impl<M: Send + Sync> Widget<M> for MyWidget {
///     fn render(&self, chunk: &mut Chunk) {
///         SimpleWidget::render(self, chunk)
///     }
///
///     fn handle_event(&mut self, _event: &Event) -> EventResult<M> {
///         EventResult::Ignored
///     }
///
///     fn constraints(&self) -> Constraints {
///         SimpleWidget::constraints(self)
///     }
/// }
/// ```
///
/// ## Pattern 2: Use Enhanced wrapper for interactivity
///
/// ```
/// use tui::prelude::*;
///
/// struct MyWidget {
///     text: String,
/// }
///
/// impl<M: Send + Sync> SimpleWidget<M> for MyWidget {
///     // ... same as above
/// }
///
/// impl<M: Send + Sync> Widget<M> for MyWidget {
///     // ... same boilerplate as above
/// }
///
/// // Add interactivity with enhanced()
/// let interactive_widget = enhanced(MyWidget { text: "Click me".into() })
///     .focusable()
///     .on_click(|| Message::Clicked);
/// ```
pub trait SimpleWidget<M>: Send + Sync {
    /// Render the widget into the provided chunk.
    ///
    /// The widget should draw at relative coordinates (0, 0) within the chunk.
    fn render(&self, chunk: &mut render::chunk::Chunk);

    /// Return the size constraints for this widget.
    ///
    /// Used by layout manager to calculate widget positions.
    fn constraints(&self) -> Constraints;
}

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

    /// Returns an optional stable key for this widget.
    ///
    /// Keys are used to maintain widget identity across tree rebuilds, especially
    /// important for dynamic lists where path alone might not be stable.
    ///
    /// # When to use keys
    ///
    /// - **Static layouts**: Not needed, path is stable enough
    /// - **Dynamic lists**: Provide unique keys (e.g., database ID, unique identifier)
    /// - **Reorderable items**: Keys maintain identity when order changes
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // For a list item widget
    /// impl<M> Widget<M> for UserListItem {
    ///     fn widget_key(&self) -> Option<&str> {
    ///         Some(&self.user_id) // Stable across reorders
    ///     }
    /// }
    /// ```
    ///
    /// # Default
    /// Returns `None` - most widgets don't need explicit keys
    fn widget_key(&self) -> Option<&str> {
        None
    }

    /// Build focus chain recursively (for containers)
    ///
    /// Allows containers to contribute their children's focus paths to the focus chain.
    /// Called during focus chain building to support nested containers.
    ///
    /// # Arguments
    /// * `current_path` - Current path in the widget tree
    /// * `chain` - Accumulated focus chain (ordered list of widget IDs)
    /// * `registry` - Widget registry (maps widget IDs to their paths)
    ///
    /// # Default
    /// Does nothing - only containers need to implement this.
    fn build_focus_chain_recursive(
        &self,
        _current_path: &mut Vec<usize>,
        _chain: &mut Vec<crate::widget_id::WidgetId>,
        _registry: &mut crate::focus::WidgetRegistry,
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
    /// * `focus_id` - The ID of the focused widget (if any)
    ///
    /// # Default
    /// Does nothing - only containers need to implement this.
    fn update_focus_states_recursive(
        &mut self,
        _current_path: &[usize],
        _focus_id: Option<crate::widget_id::WidgetId>,
    ) {
        // Default: no-op (leaf widgets don't have children)
    }
}

// Blanket implementation for Box<dyn Widget<M>>
// This allows widgets to be stored and used as boxed trait objects
impl<M> Widget<M> for Box<dyn Widget<M>> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        (**self).render(chunk)
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        (**self).handle_event(event)
    }

    fn constraints(&self) -> Constraints {
        (**self).constraints()
    }

    fn focusable(&self) -> bool {
        (**self).focusable()
    }

    fn is_focused(&self) -> bool {
        (**self).is_focused()
    }

    fn set_focused(&mut self, focused: bool) {
        (**self).set_focused(focused)
    }

    fn widget_key(&self) -> Option<&str> {
        (**self).widget_key()
    }

    fn build_focus_chain_recursive(
        &self,
        current_path: &mut Vec<usize>,
        chain: &mut Vec<crate::widget_id::WidgetId>,
        registry: &mut crate::focus::WidgetRegistry,
    ) {
        (**self).build_focus_chain_recursive(current_path, chain, registry)
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_id: Option<crate::widget_id::WidgetId>,
    ) {
        (**self).update_focus_states_recursive(current_path, focus_id)
    }
}
