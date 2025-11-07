//! Prelude module for convenient imports
//!
//! Import everything you need with `use tui::prelude::*;`

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
pub use crate::layout::{Constraints, Container, Direction};
pub use crate::style::{BorderStyle, Color, CssError, Padding, Style, TextModifiers};
pub use crate::widget::{
    AnyWidget, Button, Checkbox, KeyboardController, Label, Spacer, TextInput, Widget,
};
