//! Prelude module for convenient imports
//!
//! Import everything you need with `use tui::prelude::*;`
//!
//! # Available Functions
//!
//! When you import the prelude, you get access to these convenience functions:
//! - `col()` - Create empty vertical containers
//! - `row()` - Create empty horizontal containers
//! - `label()` - Create label widgets
//! - `keyboard_controller()` - Create keyboard controller widgets

pub use crate::app::App;
pub use crate::error::{Result, WidgetError};
pub use crate::event::{
    Event, EventResult, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use crate::layout::{col, row, Constraints, Container, Direction};
pub use crate::style::{BorderStyle, Color, CssError, Padding, Style, TextModifiers};
pub use crate::widget::{
    keyboard_controller, label, IntoWidget, KeyboardController, Label, Widget,
};
