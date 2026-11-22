//! Build applications with [`app::App`], compose [`widgets`] with [`layout`], and
//! schedule background work through [`effect::UpdateCtx`]. [`runtime`] supports
//! both terminal execution and deterministic headless stepping. [`widget`] is
//! the extension point for custom widgets; [`prelude`] collects common imports.

pub mod animation;
pub mod app;
pub mod effect;
pub mod error;
pub mod event;
pub mod focus;
pub mod gesture;
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

pub use error::{WidgetError, WidgetResult};
pub use rsille_render::InlineMouseMode;

pub mod scope;
pub mod subscription;
