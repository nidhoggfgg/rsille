//! Layout system for widget positioning

pub mod border_renderer;
pub mod constraints;
pub mod flex;
mod grid;
pub mod grid_placement;
pub mod grid_track;
mod layout;
pub mod overlay;
pub mod taffy_bridge;

pub use constraints::Constraints;
pub use flex::{col, row, Direction, Flex};
pub use grid::{grid, Grid};
pub use grid_placement::{GridLine, GridPlacement};
pub use grid_track::GridTrack;
pub use layout::Layout;
pub use overlay::{OverlayInfo, OverlayManager};
pub use taffy::style::{AlignItems, JustifyContent, JustifyItems};

/// Defines how a layout handles content that overflows its bounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    /// Content is clipped to the layout's bounds (default)
    #[default]
    Hidden,
    /// Content can overflow and render outside the layout's bounds
    Visible,
}
