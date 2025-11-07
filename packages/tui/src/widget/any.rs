//! Type-safe heterogeneous widget container

use super::*;
use crate::layout::Container;

/// Macro to reduce repetitive match dispatching in AnyWidget
macro_rules! dispatch_widget_method {
    // With arguments
    ($self:expr, $method:ident, $($args:expr),+) => {
        match $self {
            AnyWidget::Label(w) => w.$method($($args),*),
            AnyWidget::Spacer(w) => w.$method($($args),*),
            AnyWidget::Button(w) => w.$method($($args),*),
            AnyWidget::KeyboardController(w) => w.$method($($args),*),
            AnyWidget::TextInput(w) => w.$method($($args),*),
            AnyWidget::Checkbox(w) => w.$method($($args),*),
            AnyWidget::ProgressBar(w) => w.$method($($args),*),
            AnyWidget::Container(w) => w.$method($($args),*),
        }
    };
    // Without arguments
    ($self:expr, $method:ident) => {
        match $self {
            AnyWidget::Label(w) => w.$method(),
            AnyWidget::Spacer(w) => w.$method(),
            AnyWidget::Button(w) => w.$method(),
            AnyWidget::KeyboardController(w) => w.$method(),
            AnyWidget::TextInput(w) => w.$method(),
            AnyWidget::Checkbox(w) => w.$method(),
            AnyWidget::ProgressBar(w) => w.$method(),
            AnyWidget::Container(w) => w.$method(),
        }
    };
}

/// Type-safe heterogeneous widget container.
///
/// Used by Container to hold mixed widget types.
#[derive(Debug)]
pub enum AnyWidget<M = ()> {
    Label(label::Label),
    Spacer(spacer::Spacer),
    Button(button::Button<M>),
    KeyboardController(keyboard_controller::KeyboardController<M>),
    TextInput(text_input::TextInput<M>),
    Checkbox(checkbox::Checkbox<M>),
    ProgressBar(progress_bar::ProgressBar),
    Container(Container<M>),
}

impl<M> AnyWidget<M> {
    /// Render the widget
    pub fn render(&self, chunk: &mut render::chunk::Chunk)
    where
        M: Clone,
    {
        dispatch_widget_method!(self, render, chunk)
    }

    /// Get widget constraints
    pub fn constraints(&self) -> Constraints
    where
        M: Clone,
    {
        dispatch_widget_method!(self, constraints)
    }

    /// Handle event and collect any generated messages
    pub fn handle_event_with_messages(&mut self, event: &Event) -> (EventResult<M>, Vec<M>)
    where
        M: Clone,
    {
        match self {
            AnyWidget::Label(w) => {
                let _ = w.handle_event(event);
                (EventResult::Ignored, vec![])
            }
            AnyWidget::Spacer(w) => {
                let _ = w.handle_event(event);
                (EventResult::Ignored, vec![])
            }
            AnyWidget::ProgressBar(w) => {
                let _ = w.handle_event(event);
                (EventResult::Ignored, vec![])
            }
            AnyWidget::Button(w) => {
                let result = w.handle_event(event);
                let messages = result.messages_ref().to_vec();
                (result, messages)
            }
            AnyWidget::KeyboardController(w) => {
                let result = w.handle_event(event);
                let messages = result.messages_ref().to_vec();
                (result, messages)
            }
            AnyWidget::TextInput(w) => {
                let result = w.handle_event(event);
                let messages = result.messages_ref().to_vec();
                (result, messages)
            }
            AnyWidget::Checkbox(w) => {
                let result = w.handle_event(event);
                let messages = result.messages_ref().to_vec();
                (result, messages)
            }
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

impl<M> From<spacer::Spacer> for AnyWidget<M> {
    fn from(w: spacer::Spacer) -> Self {
        AnyWidget::Spacer(w)
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
