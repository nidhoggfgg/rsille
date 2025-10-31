//! Type-safe heterogeneous widget container

use super::*;
use crate::layout::Container;

/// Type-safe heterogeneous widget container.
///
/// Used by Container to hold mixed widget types.
#[derive(Debug)]
pub enum AnyWidget<M = ()> {
    Label(label::Label),
    Button(button::Button<M>),
    KeyboardController(keyboard_controller::KeyboardController<M>),
    TextInput(text_input::TextInput<M>),
    Checkbox(checkbox::Checkbox<M>),
    List(list::List),
    ProgressBar(progress_bar::ProgressBar),
    Container(Container<M>),
}

impl<M> AnyWidget<M> {
    /// Get a reference to the underlying widget
    pub fn as_widget(&self) -> &dyn Widget {
        match self {
            AnyWidget::Label(w) => w as &dyn Widget,
            AnyWidget::Button(w) => w as &dyn Widget,
            AnyWidget::KeyboardController(w) => w as &dyn Widget,
            AnyWidget::TextInput(w) => w as &dyn Widget,
            AnyWidget::Checkbox(w) => w as &dyn Widget,
            AnyWidget::List(w) => w as &dyn Widget,
            AnyWidget::ProgressBar(w) => w as &dyn Widget,
            AnyWidget::Container(w) => w as &dyn Widget,
        }
    }

    /// Get a mutable reference to the underlying widget
    pub fn as_widget_mut(&mut self) -> &mut dyn Widget {
        match self {
            AnyWidget::Label(w) => w as &mut dyn Widget,
            AnyWidget::Button(w) => w as &mut dyn Widget,
            AnyWidget::KeyboardController(w) => w as &mut dyn Widget,
            AnyWidget::TextInput(w) => w as &mut dyn Widget,
            AnyWidget::Checkbox(w) => w as &mut dyn Widget,
            AnyWidget::List(w) => w as &mut dyn Widget,
            AnyWidget::ProgressBar(w) => w as &mut dyn Widget,
            AnyWidget::Container(w) => w as &mut dyn Widget,
        }
    }

    /// Set focus state on the widget (for focus management and testing)
    pub fn set_focused(&mut self, focused: bool) {
        match self {
            AnyWidget::Label(_) => {} // Labels are not focusable
            AnyWidget::Button(w) => w.set_focused(focused),
            AnyWidget::KeyboardController(w) => w.set_focused(focused),
            AnyWidget::TextInput(w) => w.set_focused(focused),
            AnyWidget::Checkbox(w) => w.set_focused(focused),
            AnyWidget::List(_) => {}        // Lists handle focus internally
            AnyWidget::ProgressBar(_) => {} // Progress bars are not focusable
            AnyWidget::Container(_) => {}   // Containers don't hold focus directly
        }
    }

    /// Handle event and collect any generated messages
    pub fn handle_event_with_messages(&mut self, event: &Event) -> (EventResult, Vec<M>)
    where
        M: Clone,
    {
        match self {
            AnyWidget::Label(w) => (w.handle_event(event), vec![]),
            AnyWidget::Button(w) => {
                let result = w.handle_event(event);
                let messages = if result.is_consumed() {
                    w.take_message().into_iter().collect()
                } else {
                    vec![]
                };
                (result, messages)
            }
            AnyWidget::KeyboardController(w) => {
                let result = w.handle_event(event);
                let messages = if result.is_consumed() {
                    w.take_message().into_iter().collect()
                } else {
                    vec![]
                };
                (result, messages)
            }
            AnyWidget::TextInput(w) => {
                let result = w.handle_event(event);
                let messages = if result.is_consumed() {
                    w.take_message().into_iter().collect()
                } else {
                    vec![]
                };
                (result, messages)
            }
            AnyWidget::Checkbox(w) => {
                let result = w.handle_event(event);
                let messages = if result.is_consumed() {
                    w.take_message().into_iter().collect()
                } else {
                    vec![]
                };
                (result, messages)
            }
            AnyWidget::List(w) => (w.handle_event(event), vec![]),
            AnyWidget::ProgressBar(w) => (w.handle_event(event), vec![]),
            AnyWidget::Container(w) => w.handle_event_with_messages(event),
        }
    }
}

// From implementations for ergonomic conversion
impl<M> From<label::Label> for AnyWidget<M> {
    fn from(w: label::Label) -> Self {
        AnyWidget::Label(w)
    }
}

impl<M> From<button::Button<M>> for AnyWidget<M> {
    fn from(w: button::Button<M>) -> Self {
        AnyWidget::Button(w)
    }
}

impl<M> From<keyboard_controller::KeyboardController<M>> for AnyWidget<M> {
    fn from(w: keyboard_controller::KeyboardController<M>) -> Self {
        AnyWidget::KeyboardController(w)
    }
}

impl<M> From<text_input::TextInput<M>> for AnyWidget<M> {
    fn from(w: text_input::TextInput<M>) -> Self {
        AnyWidget::TextInput(w)
    }
}

impl<M> From<checkbox::Checkbox<M>> for AnyWidget<M> {
    fn from(w: checkbox::Checkbox<M>) -> Self {
        AnyWidget::Checkbox(w)
    }
}

impl<M> From<list::List> for AnyWidget<M> {
    fn from(w: list::List) -> Self {
        AnyWidget::List(w)
    }
}

impl<M> From<progress_bar::ProgressBar> for AnyWidget<M> {
    fn from(w: progress_bar::ProgressBar) -> Self {
        AnyWidget::ProgressBar(w)
    }
}

impl<M> From<Container<M>> for AnyWidget<M> {
    fn from(w: Container<M>) -> Self {
        AnyWidget::Container(w)
    }
}
