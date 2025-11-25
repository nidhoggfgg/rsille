//! Layout system for widget positioning

pub mod border_renderer;
pub mod constraints;
pub mod container;
pub mod grid;
pub mod grid_placement;
pub mod grid_track;
pub mod overlay;
pub mod taffy_bridge;

pub use constraints::Constraints;
pub use container::{col, row, Container, Direction};
pub use grid::{grid, Grid};
pub use grid_placement::{GridLine, GridPlacement};
pub use grid_track::GridTrack;
pub use overlay::{OverlayInfo, OverlayManager};
pub use taffy::style::{AlignItems, JustifyContent, JustifyItems};

/// Defines how a container handles content that overflows its bounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    /// Content is clipped to the container's bounds (default)
    #[default]
    Hidden,
    /// Content can overflow and render outside the container's bounds
    Visible,
}
