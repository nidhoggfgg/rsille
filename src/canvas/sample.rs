//! Per-sample coverage and color storage inside a terminal cell.

use crossterm::style::Color;

/// Occupied sample. An absent color inherits the terminal foreground; an absent
/// Sample is uncovered. Rows are stored from bottom to top in Cartesian order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sample {
    /// Explicit ink color, or `None` to request inherited foreground.
    pub color: Option<Color>,
}

/// Storage for up to eight samples, ordered left to right and bottom to top.
///
/// `None` means uncovered. Half-block uses the first two slots, Quadrant the
/// first four, and Braille all eight. Unused slots remain empty.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Samples(
    /// Sample slots in Cartesian row-major order.
    pub [Option<Sample>; 8],
);
