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
//! - `button()` - Create button widgets
//! - `spacer()` - Create spacer widgets
//! - `keyboard_controller()` - Create keyboard controller widgets
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

pub use crate::animation::{
    AnimatedValue, Animation, AnimationController, AnimationState, ColorInterpolate,
    ColorTransition, Easing, Interpolate, OpacityTransition, PositionTransition, ScaleTransition,
    Transition, TransitionType,
};
pub use crate::app::App;
pub use crate::error::{Result, WidgetError};
pub use crate::event::{
    Event, EventResult, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use crate::layout::{col, row, Constraints, Container, Direction};
pub use crate::style::{BorderStyle, Color, CssError, Padding, Style, TextModifiers};
pub use crate::widget::{
    keyboard_controller, label, spacer, IntoWidget, KeyboardController, Label, Spacer, Widget,
};
