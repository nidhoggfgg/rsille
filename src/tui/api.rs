pub mod animation;
pub mod app;
pub mod effect;
pub mod error;
pub mod event;
pub mod focus;
pub mod layout;
pub(crate) mod offscreen;
pub mod shell;
pub mod state;
pub mod style;
pub mod text;
pub(crate) mod visual_engine;
pub mod widget;
pub mod widgets;

pub mod prelude;
pub mod runtime;

pub use crate::render::InlineMouseMode;
pub use error::{WidgetError, WidgetResult};

pub mod scope;
pub mod subscription;
