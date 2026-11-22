//! Zero-based positions, sizes, and rectangles measured in terminal cells.
//!
//! Coordinates grow to the right and downward. Areas use exclusive right and
//! bottom edges; their containment helpers distinguish local coordinates from
//! absolute rectangles.

#[allow(clippy::module_inception)]
mod area;
mod pos;
mod size;

pub use area::Area;
pub use pos::Position;
pub use size::Size;
