pub use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Terminal(TerminalEvent),
    Local(LocalEvent),
}

impl Event {
    pub fn from(event: crossterm::event::Event) -> Self {
        match event {
            crossterm::event::Event::Key(key) => Self::Key(key),
            crossterm::event::Event::Mouse(mouse) => Self::Mouse(mouse),
            crossterm::event::Event::Resize(width, height) => {
                Self::Terminal(TerminalEvent::Resize(width, height))
            }
            crossterm::event::Event::FocusGained => Self::Terminal(TerminalEvent::FocusGained),
            crossterm::event::Event::FocusLost => Self::Terminal(TerminalEvent::FocusLost),
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerminalEvent {
    FocusGained,
    FocusLost,
    Resize(u16, u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum LocalEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}
