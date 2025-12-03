//! Layout trait for root-level widgets

use crate::event::{Event, EventResult};
use crate::widget_id::WidgetId;
use crate::layout::Constraints;
use crate::widget::Widget;

/// Layout trait for widgets that can be used as the root of a UI tree
///
/// This trait is implemented by Flex and Grid, allowing either to be used
/// as the return type of view functions in App::run.
pub trait Layout<M: Clone>: Widget<M> {
    /// Build focus chain by recursively collecting focusable widgets
    fn build_focus_chain(&self, current_path: &mut Vec<usize>, chain: &mut Vec<WidgetId>);

    /// Update focus state for all children based on focus ID
    fn update_focus_states(&mut self, current_path: &[usize], focus_id: Option<WidgetId>);

    /// Handle event with focus information
    fn handle_event_with_focus(
        &mut self,
        event: &Event,
        current_path: &[usize],
        focus_id: Option<WidgetId>,
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
        chain: &mut Vec<WidgetId>,
    ) {
        (**self).build_focus_chain_recursive(current_path, chain)
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_id: Option<WidgetId>,
    ) {
        (**self).update_focus_states_recursive(current_path, focus_id)
    }
}

impl<M: Clone> Layout<M> for Box<dyn Layout<M>> {
    fn build_focus_chain(&self, current_path: &mut Vec<usize>, chain: &mut Vec<WidgetId>) {
        (**self).build_focus_chain(current_path, chain)
    }

    fn update_focus_states(&mut self, current_path: &[usize], focus_id: Option<WidgetId>) {
        (**self).update_focus_states(current_path, focus_id)
    }

    fn handle_event_with_focus(
        &mut self,
        event: &Event,
        current_path: &[usize],
        focus_id: Option<WidgetId>,
    ) -> (EventResult<M>, Vec<M>) {
        (**self).handle_event_with_focus(event, current_path, focus_id)
    }
}
