//! Layout containers, size proposals, and geometry for terminal widgets.
//!
//! Dimensions, gaps, padding, and offsets use terminal cells: widths are columns
//! and heights are rows. The published API is `rsille::tui::layout`.
//!
//! # Choosing a container
//!
//! | Layout | Entry point | Behavior |
//! | --- | --- | --- |
//! | Linear | [`row`], [`col`] | Arrange children using flexbox. |
//! | Two-dimensional | [`grid()`] | Size tracks and place children by grid lines. |
//! | Shared area | [`stack()`] | Paint children in insertion order in the same area. |
//! | Local popups | [`overlay()`] | Position layers within a base widget's allocation. |
//! | Root popups | [`portal()`] | Place layers over the root, optionally anchored to widget identities. |
//! | Scrolling | [`scroll_view()`], [`scroll_lines`] | Clip larger content to an interactive viewport. |
//! | Two panes | [`split()`] | Divide space with an interactive resize handle. |
//! | Floating windows | [`window_host()`] | Constrain, activate, and stack keyed windows. |
//!
//! Builders accept widgets through [`IntoWidget`](crate::tui::widget::IntoWidget).
//! Their message type `M` matches the application; use `::<()>` for views that do
//! not emit messages. Give stateful containers stable keys when their position
//! in the widget tree may change.
//!
//! # Example
//!
//! ```
//! use rsille::tui::{layout::{col, row, scroll_lines, ScrollView}, widgets::label};
//!
//! let log: ScrollView<()> = scroll_lines(["Connected", "Ready"])
//!     .key("log")
//!     .focusable(true);
//! let view = col::<()>()
//!     .gap(1)
//!     .child(row().gap(2).child(label("Status")).child(label("Online")))
//!     .child(log);
//! assert_eq!(view.len(), 2);
//! ```
//!
//! # Implementing containers
//!
//! Parents pass a [`SizeProposal`] to
//! [`Widget::measure`](crate::tui::widget::Widget::measure), which returns a
//! [`MeasuredSize`]. [`LayoutStyle`] describes size limits and flex preferences;
//! [`Constraints`] supplies the simpler, compatible sizing API. Measurement
//! estimates size, while [`Layout`] records final [`ChildLayout`] allocations
//! in paint order. A layout can also carry container-specific geometry so
//! painting can reuse it without repeating measurement.
//!
//! [`LayoutGeometry`] reports the last painted target, displayed, clipped, and
//! hit-test areas. [`Invalidation`] distinguishes layout, paint, and next-frame
//! requests, and [`LayoutStats`] counts layout computations and cache reuse.
//! [`taffy_bridge`] and [`border_renderer`] expose lower-level helpers for custom
//! containers.

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

pub mod portal;
pub use portal::{portal, Portal, PortalLayer};

pub mod window_host;
pub use window_host::{window_host, WindowHost, WindowHostState};
