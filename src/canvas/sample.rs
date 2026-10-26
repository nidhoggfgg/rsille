use crossterm::style::Color;

/// Occupied sample. An absent color inherits the terminal foreground; an absent
/// Sample is uncovered. Rows are stored from bottom to top in Cartesian order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sample {
    pub color: Option<Color>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Samples(pub [Option<Sample>; 8]);
