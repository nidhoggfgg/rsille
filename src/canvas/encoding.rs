/// How a canvas's samples are encoded into terminal characters.
///
/// Select the encoding when constructing a canvas. Sample coordinates depend on
/// this choice; world-coordinate drawings can share a viewport across encodings.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Encoding {
    /// Eight samples per cell, useful for curves, wireframes and particles.
    #[default]
    Braille,
    /// Two vertically stacked samples per cell, each with its own color. When
    /// the text-width policy makes block characters wide, drawing falls back to
    /// single-column Braille equivalents of the upper/lower/full blocks.
    HalfBlock,
}

impl Encoding {
    /// Sample columns and rows represented by one terminal cell.
    pub const fn samples_per_cell(self) -> (u16, u16) {
        match self {
            Self::Braille => (2, 4),
            Self::HalfBlock => (1, 2),
        }
    }
}
