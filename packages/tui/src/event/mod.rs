//! Event system for handling user input

pub mod focus;
pub mod handler;
pub mod result;

pub use focus::{FocusManager, WidgetId};
pub use handler::{EventEmitter, EventHandler, EventHandlerWithContext};
pub use result::EventResult;

pub use KeyCode::*; // Re-export key codes for convenience
// Don't re-export MouseButton::* to avoid conflicts with KeyCode (Left, Right)

/// Event types that widgets can handle
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input
    Key(KeyEvent),
    /// Mouse input
    Mouse(MouseEvent),
    /// Terminal resized
    Resize(u16, u16),
    /// Frame tick for animations
    Tick,
}

/// Mouse event details
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: Modifiers,
}

impl MouseEvent {
    pub fn new(kind: MouseEventKind, column: u16, row: u16) -> Self {
        Self {
            kind,
            column,
            row,
            modifiers: Modifiers::empty(),
        }
    }

    pub fn with_modifiers(kind: MouseEventKind, column: u16, row: u16, modifiers: Modifiers) -> Self {
        Self {
            kind,
            column,
            row,
            modifiers,
        }
    }
}

/// Mouse event kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    /// Mouse button pressed
    Down(MouseButton),
    /// Mouse button released
    Up(MouseButton),
    /// Mouse moved with button pressed
    Drag(MouseButton),
    /// Mouse moved without button pressed
    Moved,
    /// Mouse wheel scrolled
    ScrollDown,
    ScrollUp,
}

/// Mouse buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Keyboard event details
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: Modifiers,
}

impl KeyEvent {
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: Modifiers::empty(),
        }
    }

    pub fn with_modifiers(code: KeyCode, modifiers: Modifiers) -> Self {
        Self { code, modifiers }
    }
}

/// Key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Tab,
    BackTab,
    Backspace,
    Esc,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
    F(u8),
    Space,
    Null,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
    bits: u8,
}

impl Modifiers {
    pub const SHIFT: u8 = 0b0001;
    pub const CONTROL: u8 = 0b0010;
    pub const ALT: u8 = 0b0100;
    pub const META: u8 = 0b1000;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn shift() -> Self {
        Self { bits: Self::SHIFT }
    }

    pub const fn ctrl() -> Self {
        Self { bits: Self::CONTROL }
    }

    pub const fn alt() -> Self {
        Self { bits: Self::ALT }
    }

    pub const fn with_shift(mut self) -> Self {
        self.bits |= Self::SHIFT;
        self
    }

    pub const fn with_control(mut self) -> Self {
        self.bits |= Self::CONTROL;
        self
    }

    pub const fn with_alt(mut self) -> Self {
        self.bits |= Self::ALT;
        self
    }

    pub const fn contains_shift(&self) -> bool {
        self.bits & Self::SHIFT != 0
    }

    pub const fn contains_ctrl(&self) -> bool {
        self.bits & Self::CONTROL != 0
    }

    pub const fn contains_alt(&self) -> bool {
        self.bits & Self::ALT != 0
    }
}

// Allow bitwise OR for combining modifiers
impl std::ops::BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}

impl std::ops::BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}
