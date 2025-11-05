use crate::area::{Position, Size};
use std::io;

/// Errors that can occur during drawing and rendering operations
#[derive(Debug)]
pub enum DrawErr {
    /// Position is out of bounds
    OutOfBounds { pos: Position, bounds: Size },
    /// Position is already occupied by another character
    PositionOccupied { pos: Position },
    /// Invalid area configuration
    InvalidArea { area_size: Size, buffer_size: Size },
    /// Invalid shrink parameters
    InvalidShrink {
        current_size: Size,
        shrink_total: Size,
    },
    /// Content width exceeds available space
    ContentTooWide {
        content_width: u16,
        available_width: u16,
        x: u16,
    },
    /// Terminal setup operation failed (enter alt screen, enable raw mode, etc.)
    TerminalSetup(io::Error),
    /// Terminal cleanup operation failed (leave alt screen, disable raw mode, etc.)
    TerminalCleanup(io::Error),
    /// Thread panicked during execution
    ThreadPanic(String),
    /// Channel was closed unexpectedly
    ChannelClosed,
    /// Failed to create tokio runtime
    RuntimeCreation(io::Error),
    /// IO error during rendering
    Io(io::Error),
}

impl DrawErr {
    /// Create an OutOfBounds error
    pub fn out_of_bounds(pos: Position, bounds: Size) -> Self {
        Self::OutOfBounds { pos, bounds }
    }

    /// Create a PositionOccupied error
    pub fn position_occupied(pos: Position) -> Self {
        Self::PositionOccupied { pos }
    }

    /// Create an InvalidArea error
    pub fn invalid_area(area_size: Size, buffer_size: Size) -> Self {
        Self::InvalidArea {
            area_size,
            buffer_size,
        }
    }

    /// Create an InvalidShrink error
    pub fn invalid_shrink(current_size: Size, shrink_total: Size) -> Self {
        Self::InvalidShrink {
            current_size,
            shrink_total,
        }
    }

    /// Create a ContentTooWide error
    pub fn content_too_wide(content_width: u16, available_width: u16, x: u16) -> Self {
        Self::ContentTooWide {
            content_width,
            available_width,
            x,
        }
    }

    /// Create a ThreadPanic error from a panic payload
    pub fn thread_panic(payload: Box<dyn std::any::Any + Send>) -> Self {
        let panic_msg = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Thread panicked with unknown payload".to_string()
        };
        Self::ThreadPanic(panic_msg)
    }

    /// Get detailed context about this error
    pub fn context(&self) -> String {
        match self {
            Self::OutOfBounds { pos, bounds } => {
                format!(
                    "Position ({}, {}) exceeds bounds: x must be < {}, y must be < {}",
                    pos.x, pos.y, bounds.width, bounds.height
                )
            }
            Self::PositionOccupied { pos } => {
                format!(
                    "Position ({}, {}) is occupied by a wide character",
                    pos.x, pos.y
                )
            }
            Self::InvalidArea {
                area_size,
                buffer_size,
            } => {
                format!(
                    "Area ({}×{}) exceeds buffer ({}×{}): overflow of {}×{}",
                    area_size.width,
                    area_size.height,
                    buffer_size.width,
                    buffer_size.height,
                    area_size.width.saturating_sub(buffer_size.width),
                    area_size.height.saturating_sub(buffer_size.height)
                )
            }
            Self::InvalidShrink {
                current_size,
                shrink_total,
            } => {
                format!(
                    "Cannot shrink ({}×{}) by ({}×{}): would result in negative size",
                    current_size.width,
                    current_size.height,
                    shrink_total.width,
                    shrink_total.height
                )
            }
            Self::ContentTooWide {
                content_width,
                available_width,
                x,
            } => {
                format!(
                    "Content width {} at x={} exceeds available width {} (overflow: {})",
                    content_width,
                    x,
                    available_width,
                    (x + content_width).saturating_sub(*available_width)
                )
            }
            Self::TerminalSetup(err) => format!("Terminal setup failed: {}", err),
            Self::TerminalCleanup(err) => format!("Terminal cleanup failed: {}", err),
            Self::ThreadPanic(msg) => format!("Thread panicked: {}", msg),
            Self::ChannelClosed => "Communication channel closed unexpectedly".to_string(),
            Self::RuntimeCreation(err) => format!("Failed to create tokio runtime: {}", err),
            Self::Io(err) => format!("IO error: {}", err),
        }
    }
}

impl From<DrawErr> for std::io::Error {
    fn from(value: DrawErr) -> Self {
        match value {
            DrawErr::Io(e) => e,
            DrawErr::TerminalSetup(e) => e,
            DrawErr::TerminalCleanup(e) => e,
            DrawErr::RuntimeCreation(e) => e,
            other => std::io::Error::other(other),
        }
    }
}

impl From<io::Error> for DrawErr {
    fn from(err: io::Error) -> Self {
        DrawErr::Io(err)
    }
}

impl std::fmt::Display for DrawErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrawErr::OutOfBounds { pos, bounds } => {
                write!(
                    f,
                    "Out of bounds: position ({}, {}) exceeds buffer size ({}×{})",
                    pos.x, pos.y, bounds.width, bounds.height
                )
            }
            DrawErr::PositionOccupied { pos } => {
                write!(
                    f,
                    "Position occupied: ({}, {}) is already occupied by a wide character",
                    pos.x, pos.y
                )
            }
            DrawErr::InvalidArea {
                area_size,
                buffer_size,
            } => {
                write!(
                    f,
                    "Invalid area: area size ({}×{}) exceeds buffer size ({}×{})",
                    area_size.width, area_size.height, buffer_size.width, buffer_size.height
                )
            }
            DrawErr::InvalidShrink {
                current_size,
                shrink_total,
            } => {
                write!(
                    f,
                    "Invalid shrink: shrink amount ({}×{}) exceeds current size ({}×{})",
                    shrink_total.width,
                    shrink_total.height,
                    current_size.width,
                    current_size.height
                )
            }
            DrawErr::ContentTooWide {
                content_width,
                available_width,
                x,
            } => {
                write!(
                    f,
                    "Content too wide: width {} at x={} exceeds available width {}",
                    content_width, x, available_width
                )
            }
            DrawErr::TerminalSetup(err) => write!(f, "Terminal setup failed: {}", err),
            DrawErr::TerminalCleanup(err) => write!(f, "Terminal cleanup failed: {}", err),
            DrawErr::ThreadPanic(msg) => write!(f, "Thread panicked: {}", msg),
            DrawErr::ChannelClosed => write!(f, "Communication channel closed unexpectedly"),
            DrawErr::RuntimeCreation(err) => write!(f, "Failed to create tokio runtime: {}", err),
            DrawErr::Io(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl core::error::Error for DrawErr {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            DrawErr::TerminalSetup(err) => Some(err),
            DrawErr::TerminalCleanup(err) => Some(err),
            DrawErr::RuntimeCreation(err) => Some(err),
            DrawErr::Io(err) => Some(err),
            _ => None,
        }
    }
}
