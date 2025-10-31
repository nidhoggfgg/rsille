//! Error types for the TUI framework

use thiserror::Error;

/// Result type alias for TUI operations
pub type Result<T> = std::result::Result<T, WidgetError>;

/// Errors that can occur during widget operations
#[derive(Debug, Error)]
pub enum WidgetError {
    #[error("Invalid widget configuration: {0}")]
    InvalidConfig(String),

    #[error("Circular containment detected")]
    CircularContainment,

    #[error("Widget not found: {0}")]
    WidgetNotFound(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("Layout error: {0}")]
    LayoutError(String),

    #[error("Event handling error: {0}")]
    EventError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
