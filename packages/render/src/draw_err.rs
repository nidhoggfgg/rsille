use crate::area::{Position, Size};

/// Errors that can occur during drawing operations
#[derive(Debug, Clone, PartialEq, Eq)]
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
        }
    }
}

impl From<DrawErr> for std::io::Error {
    fn from(value: DrawErr) -> Self {
        std::io::Error::other(value)
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
        }
    }
}

impl core::error::Error for DrawErr {}
