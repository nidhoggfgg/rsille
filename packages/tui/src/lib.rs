pub mod app;
pub mod error;
pub mod event;
pub mod focus;
pub mod layout;
pub mod style;
pub mod widget;

// Declarative UI macro
pub mod ui_macro;

// Convenience re-exports
pub mod prelude;

pub use error::{Result, WidgetError};
