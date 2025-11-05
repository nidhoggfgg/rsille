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
    /// Render the widget
    pub fn render(&self, chunk: &mut render::chunk::Chunk, area: Rect)
    where
        M: Clone,
    {
        match self {
            AnyWidget::Label(w) => w.render(chunk, area),
            AnyWidget::Button(w) => w.render(chunk, area),
            AnyWidget::KeyboardController(w) => w.render(chunk, area),
            AnyWidget::TextInput(w) => w.render(chunk, area),
            AnyWidget::Checkbox(w) => w.render(chunk, area),
            AnyWidget::List(w) => w.render(chunk, area),
            AnyWidget::ProgressBar(w) => w.render(chunk, area),
            AnyWidget::Container(w) => w.render(chunk, area),
        }
    }

    /// Get widget constraints
    pub fn constraints(&self) -> Constraints
    where
        M: Clone,
    {
        match self {
            AnyWidget::Label(w) => w.constraints(),
            AnyWidget::Button(w) => w.constraints(),
            AnyWidget::KeyboardController(w) => w.constraints(),
            AnyWidget::TextInput(w) => w.constraints(),
            AnyWidget::Checkbox(w) => w.constraints(),
            AnyWidget::List(w) => w.constraints(),
            AnyWidget::ProgressBar(w) => w.constraints(),
            AnyWidget::Container(w) => w.constraints(),
        }
    }

    /// Check if widget is focusable
    pub fn focusable(&self) -> bool
    where
        M: Clone,
    {
        match self {
            AnyWidget::Label(w) => w.focusable(),
            AnyWidget::Button(w) => w.focusable(),
            AnyWidget::KeyboardController(w) => w.focusable(),
            AnyWidget::TextInput(w) => w.focusable(),
            AnyWidget::Checkbox(w) => w.focusable(),
            AnyWidget::List(w) => w.focusable(),
            AnyWidget::ProgressBar(w) => w.focusable(),
            AnyWidget::Container(w) => w.focusable(),
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
    ///
    /// Now simplified: messages are directly extracted from EventResult<M>
    pub fn handle_event_with_messages(&mut self, event: &Event) -> (EventResult<M>, Vec<M>)
    where
        M: Clone,
    {
        match self {
            AnyWidget::Label(w) => {
                let _result = w.handle_event(event);
                // Labels produce no messages (type is ())
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
            AnyWidget::List(w) => {
                let _result = w.handle_event(event);
                // Lists produce no messages (type is ())
                (EventResult::Ignored, vec![])
            }
            AnyWidget::ProgressBar(w) => {
                let _result = w.handle_event(event);
                // ProgressBars produce no messages (type is ())
                (EventResult::Ignored, vec![])
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
