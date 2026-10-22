//! Layout system for widget positioning

pub mod alignment;
pub mod border_renderer;
pub mod constraints;
pub mod flex;
mod grid;
pub mod grid_placement;
pub mod grid_track;
pub mod overlay;
pub mod scroll;
pub mod split;
pub mod taffy_bridge;

pub use alignment::{HorizontalAlign, VerticalAlign};
pub use constraints::{AxisLimit, Constraints, LayoutStyle, MeasuredSize, Overflow, SizeProposal};
pub use flex::{col, row, Direction, Flex};
pub use grid::{grid, Grid};
pub use grid_placement::{GridLine, GridPlacement};
pub use grid_track::GridTrack;
pub use overlay::{
    overlay, stack, AnchorRect, Overlay, OverlayAnchor, OverlayLayer, OverlayPlacement, Stack,
};
pub use scroll::{
    clamp_scroll_offset, ensure_item_visible, max_scroll_offset, scroll_lines,
    scroll_offset_for_item, scroll_view, scrollbar, ScrollAxis, ScrollState, ScrollView, Scrollbar,
    ScrollbarOrientation, ScrollbarVisibility,
};
pub use split::{split, Split, SplitDirection, SplitSize, SplitState};
pub use taffy::style::{AlignItems, JustifyContent, JustifyItems};

mod result;
pub(crate) use result::LayoutCache;
pub use result::{ChildLayout, Invalidation, Layout, LayoutGeometry, LayoutStats};
