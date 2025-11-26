//! Error types for the TUI framework

use std::fmt;

/// Result type alias for TUI operations
pub type Result<T> = std::result::Result<T, WidgetError>;

/// Errors that can occur during widget operations
#[derive(Debug)]
pub enum WidgetError {
    InvalidConfig(String),
    CircularContainment,
    WidgetNotFound(String),
    RenderError(String),
    LayoutError(String),
    EventError(String),
    Io(std::io::Error),
}

impl fmt::Display for WidgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WidgetError::InvalidConfig(msg) => write!(f, "Invalid widget configuration: {}", msg),
            WidgetError::CircularContainment => write!(f, "Circular containment detected"),
            WidgetError::WidgetNotFound(name) => write!(f, "Widget not found: {}", name),
            WidgetError::RenderError(msg) => write!(f, "Render error: {}", msg),
            WidgetError::LayoutError(msg) => write!(f, "Layout error: {}", msg),
            WidgetError::EventError(msg) => write!(f, "Event handling error: {}", msg),
            WidgetError::Io(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for WidgetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WidgetError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WidgetError {
    fn from(err: std::io::Error) -> Self {
        WidgetError::Io(err)
    }
}

impl From<taffy::TaffyError> for WidgetError {
    fn from(err: taffy::TaffyError) -> Self {
        WidgetError::LayoutError(err.to_string())
    }
}
