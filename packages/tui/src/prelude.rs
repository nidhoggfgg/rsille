//! Prelude module for convenient imports
//!
//! Import everything you need with `use tui::prelude::*;`

pub use crate::app::App;
pub use crate::buffer::Buffer;
pub use crate::error::{Result, WidgetError};
pub use crate::event::{Event, EventResult, KeyCode, KeyEvent};
pub use crate::layout::{Constraints, Container, Direction};
pub use crate::style::{BorderStyle, Color, CssError, Modifiers, Padding, Style};
pub use crate::widget::{
    AnyWidget, Button, Checkbox, KeyboardController, Label, List, ProgressBar, Rect, TextInput,
    Widget,
};
