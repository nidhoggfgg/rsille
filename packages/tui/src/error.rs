//! Error types for the TUI framework

use std::fmt;

/// Result type alias for TUI operations
pub type WidgetResult<T> = std::result::Result<T, WidgetError>;

/// Errors that can occur during widget operations
#[derive(Debug)]
pub enum WidgetError {
    /// Invalid widget configuration with details
    InvalidConfig(String),

    /// Circular containment detected in widget tree
    CircularContainment,

    /// Widget not found by name or ID
    WidgetNotFound(String),

    /// Error during widget rendering
    RenderError { message: String },

    /// Error during layout calculation
    LayoutError { source: taffy::TaffyError },

    /// Error during event handling
    EventError { message: String },

    /// IO error from underlying operations
    Io(std::io::Error),
}

impl WidgetError {
    /// Create a render error
    pub fn render_error(message: impl Into<String>) -> Self {
        Self::RenderError {
            message: message.into(),
        }
    }

    /// Create a layout error
    pub fn layout_error(source: taffy::TaffyError) -> Self {
        Self::LayoutError { source }
    }

    /// Create an event error
    pub fn event_error(message: impl Into<String>) -> Self {
        Self::EventError {
            message: message.into(),
        }
    }
}

impl fmt::Display for WidgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WidgetError::InvalidConfig(msg) => {
                write!(f, "Invalid widget configuration: {}", msg)
            }
            WidgetError::CircularContainment => {
                write!(f, "Circular containment detected")
            }
            WidgetError::WidgetNotFound(name) => {
                write!(f, "Widget not found: {}", name)
            }
            WidgetError::RenderError { message } => {
                write!(f, "Render error: {}", message)
            }
            WidgetError::LayoutError { source } => {
                write!(f, "Layout error: {}", source)
            }
            WidgetError::EventError { message } => {
                write!(f, "Event handling error: {}", message)
            }
            WidgetError::Io(err) => {
                write!(f, "IO error: {}", err)
            }
        }
    }
}

impl std::error::Error for WidgetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WidgetError::Io(err) => Some(err),
            WidgetError::LayoutError { source, .. } => Some(source),
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
        WidgetError::layout_error(err)
    }
}
