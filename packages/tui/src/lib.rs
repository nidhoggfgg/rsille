pub mod app;
pub mod error;
pub mod event;
pub mod focus;
pub mod hover;
pub mod layout;
pub mod style;
pub mod widget;
pub mod widget_id;

// Declarative UI macro
pub mod ui_macro;

// Convenience re-exports
pub mod prelude;

pub use error::{WidgetError, WidgetResult};
