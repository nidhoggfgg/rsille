//! Terminal-cell encodings and their sample-grid dimensions.

/// How a canvas's samples are encoded into terminal characters.
///
/// Select the encoding with [`Canvas::with_encoding`](super::Canvas::with_encoding).
/// Sample coordinates depend on this choice; canvas bounds always use terminal
/// cells. The default encoding is [`Braille`](Self::Braille).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Encoding {
    /// Eight samples per cell in a 2×4 grid, useful for curves and wireframes.
    ///
    /// Partial cells preserve their dot pattern and average explicit sample
    /// colors into one foreground. Fully colored cells fit their samples to
    /// two terminal colors.
    #[default]
    Braille,
    /// Two vertically stacked samples per cell, each with its own color. When
    /// the text-width policy makes block characters wide, drawing falls back to
    /// single-column Braille equivalents of the upper/lower/full blocks.
    HalfBlock,
    /// Four samples per cell in a 2×2 grid, rendered with quadrant blocks.
    /// Partial cells preserve coverage and average their ink colors; fully
    /// colored cells fit four samples to two terminal colors. Wide block
    /// characters fall back to single-column Braille equivalents when drawn.
    Quadrant,
}

impl Encoding {
    /// Sample columns and rows represented by one terminal cell.
    ///
    /// Returns `(width, height)` in samples, rather than terminal cells.
    ///
    /// ```
    /// use rsille::canvas::Encoding;
    ///
    /// assert_eq!(Encoding::Braille.samples_per_cell(), (2, 4));
    /// assert_eq!(Encoding::HalfBlock.samples_per_cell(), (1, 2));
    /// assert_eq!(Encoding::Quadrant.samples_per_cell(), (2, 2));
    /// ```
    pub const fn samples_per_cell(self) -> (u16, u16) {
        match self {
            Self::Braille => (2, 4),
            Self::HalfBlock => (1, 2),
            Self::Quadrant => (2, 2),
        }
    }
}
