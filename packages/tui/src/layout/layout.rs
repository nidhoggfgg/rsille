//! Layout trait for root-level widgets

use crate::event::{Event, EventResult};
use crate::focus::WidgetRegistry;
use crate::layout::Constraints;
use crate::widget::Widget;
use crate::widget_id::WidgetId;

/// Layout trait for widgets that can be used as the root of a UI tree
///
/// This trait is implemented by Flex and Grid, allowing either to be used
/// as the return type of view functions in App::run.
///
/// # Focus Chain Building
///
/// Layout widgets use the standard Widget trait's `build_focus_chain_recursive`
/// method to build their focus chains. AppWrapper initializes an empty chain
/// and registry, then calls the root layout's `build_focus_chain_recursive`.
pub trait Layout<M: Clone>: Widget<M> {
    /// Update focus state for all children based on focus ID
    fn update_focus_states(&mut self, focus_id: Option<WidgetId>, registry: &WidgetRegistry);

    /// Handle event with focus information
    fn handle_event_with_focus(
        &mut self,
        event: &Event,
        focus_id: Option<WidgetId>,
        registry: &WidgetRegistry,
    ) -> (EventResult<M>, Vec<M>);
}

// Blanket implementation for Box<dyn Layout<M>>
// This allows view functions to return Box<dyn Layout<M>> when needed
impl<M: Clone> Widget<M> for Box<dyn Layout<M>> {
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
        _current_path: &[usize],
        _focus_id: Option<WidgetId>,
    ) {
        // No-op for Layout (uses update_focus_states instead)
    }
}

impl<M: Clone> Layout<M> for Box<dyn Layout<M>> {
    fn update_focus_states(&mut self, focus_id: Option<WidgetId>, registry: &WidgetRegistry) {
        (**self).update_focus_states(focus_id, registry)
    }

    fn handle_event_with_focus(
        &mut self,
        event: &Event,
        focus_id: Option<WidgetId>,
        registry: &WidgetRegistry,
    ) -> (EventResult<M>, Vec<M>) {
        (**self).handle_event_with_focus(event, focus_id, registry)
    }
}
