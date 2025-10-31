//! Size constraint types for layout calculation

/// Size constraints for layout
#[derive(Debug, Clone, Copy)]
pub struct Constraints {
    pub min_width: u16,
    pub max_width: Option<u16>,
    pub min_height: u16,
    pub max_height: Option<u16>,
    pub flex: Option<f32>,
}

impl Constraints {
    /// Create fixed-size constraints
    pub fn fixed(width: u16, height: u16) -> Self {
        Self {
            min_width: width,
            max_width: Some(width),
            min_height: height,
            max_height: Some(height),
            flex: None,
        }
    }

    /// Create flexible constraints that fill available space
    pub fn fill() -> Self {
        Self {
            min_width: 0,
            max_width: None,
            min_height: 0,
            max_height: None,
            flex: Some(1.0),
        }
    }

    /// Create minimum-size constraints
    pub fn min(width: u16, height: u16) -> Self {
        Self {
            min_width: width,
            max_width: None,
            min_height: height,
            max_height: None,
            flex: None,
        }
    }

    /// Create content-based constraints (no flex)
    pub fn content() -> Self {
        Self {
            min_width: 0,
            max_width: None,
            min_height: 0,
            max_height: None,
            flex: None,
        }
    }
}
