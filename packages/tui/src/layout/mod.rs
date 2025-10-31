//! Layout system for widget positioning

pub mod constraints;
pub mod container;
pub mod taffy_bridge;

pub use constraints::Constraints;
pub use container::{Container, Direction};

use crate::widget::common::Rect;

/// Layout manager handles widget positioning
pub struct LayoutManager {
    // Taffy integration added in US1
}

impl LayoutManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Compute layout for widgets
    pub fn compute_layout(&self, _available: Rect) -> Vec<Rect> {
        vec![]
    }
}
