//! Prelude module for convenient imports
//!
//! Import everything you need with `use tui::prelude::*;`
//!
//! # Available Functions
//!
//! When you import the prelude, you get access to these convenience functions:
//! - `col()` - Create empty vertical containers
//! - `row()` - Create empty horizontal containers
//! - `grid()` - Create grid containers
//! - `label()` - Create label widgets
//! - `button()` - Create button widgets
//! - `checkbox()` - Create checkbox widgets
//! - `checkbox_group()` - Create checkbox group widgets
//! - `radio_group()` - Create radio group widgets
//! - `text_input()` - Create text input widgets
//! - `spacer()` - Create spacer widgets
//! - `keyboard_controller()` - Create keyboard controller widgets
//! - `interactive()` - Wrap widgets with mouse event handling
//!
//! # Available Macros (Legacy)
//!
//! The old macro-based API is still available:
//! - `col![]` - Create vertical containers (macro)
//! - `row![]` - Create horizontal containers (macro)
//! - `label!()` - Create label widgets (macro)
//! - `button!()` - Create button widgets (macro)
//! - `checkbox!()` - Create checkbox widgets (macro)
//! - `text_input!()` - Create text input widgets (macro)
//! - `spacer!()` - Create spacer widgets (macro)
//! - `keyboard_controller!()` - Create keyboard controller widgets (macro)

pub use crate::app::App;
pub use crate::error::{Result, WidgetError};
pub use crate::event::{
    Event, EventResult, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use crate::layout::{col, grid, row, Constraints, Container, Direction, Grid, GridLine, GridPlacement, GridTrack};
pub use crate::style::{BorderStyle, Color, CssError, Padding, Style, TextModifiers};
pub use crate::widget::{
    button, checkbox, checkbox_group, interactive, keyboard_controller, label, radio_group,
    text_input, Button, ButtonVariant, Checkbox, CheckboxDirection, CheckboxGroup, Interactive,
    IntoWidget, KeyboardController, Label, RadioDirection, RadioGroup, TextInput, TextInputVariant,
    Widget,
};
