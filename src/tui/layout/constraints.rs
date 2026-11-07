//! Size constraint types for layout calculation

/// Per-axis size proposal supplied by a parent during measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisLimit {
    /// Requests the natural size without a parent-imposed upper bound.
    Unbounded,
    /// Requests a size no larger than the given number of terminal cells.
    AtMost(u16),
    /// Requests the given number of terminal cells.
    Exact(u16),
}

impl AxisLimit {
    /// Returns the cell count of a bounded proposal, or `None` for [`Self::Unbounded`].
    pub fn exact_or_at_most(self) -> Option<u16> {
        match self {
            AxisLimit::Unbounded => None,
            AxisLimit::AtMost(value) | AxisLimit::Exact(value) => Some(value),
        }
    }
}

/// Size proposal supplied by a parent before final layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeProposal {
    /// Width proposal in terminal columns.
    pub width: AxisLimit,
    /// Height proposal in terminal rows.
    pub height: AxisLimit,
}

impl SizeProposal {
    /// A proposal with no upper bound on either axis.
    pub const UNBOUNDED: Self = Self {
        width: AxisLimit::Unbounded,
        height: AxisLimit::Unbounded,
    };

    /// Creates a proposal requesting the given width and height in cells.
    pub fn exact(width: u16, height: u16) -> Self {
        Self {
            width: AxisLimit::Exact(width),
            height: AxisLimit::Exact(height),
        }
    }

    /// Creates a proposal with the given upper bounds in cells.
    pub fn at_most(width: u16, height: u16) -> Self {
        Self {
            width: AxisLimit::AtMost(width),
            height: AxisLimit::AtMost(height),
        }
    }
}

/// Intrinsic size returned by a widget measurement pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasuredSize {
    /// Measured width in terminal columns.
    pub width: u16,
    /// Measured height in terminal rows.
    pub height: u16,
}

impl MeasuredSize {
    /// A measurement with zero width and height.
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    /// Creates a measurement in terminal columns and rows.
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

/// Overflow preference for a widget axis.
///
/// These values express intent only: the current Taffy bridge does not apply
/// them. Use [`super::ScrollView`] to provide scrolling behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    /// Allows content to extend beyond its allocation when supported by the container.
    Visible,
    /// Requests clipping to the allocated area.
    Clip,
    /// Requests scrollable overflow; does not itself create a scroll viewport.
    Scroll,
}

/// Rich layout intent for a widget.
///
/// This is the measurement-era replacement for plain [`Constraints`]. The
/// old constraints remain the source-compatible API and are converted into
/// this richer style by default. Dimensions are terminal cells, not pixels.
/// Keep maximums at least as large as minimums and flex factors nonnegative.
/// Overflow and aspect-ratio fields are currently metadata only in the bridge.
///
/// # Examples
///
/// ```
/// use rsille::tui::layout::{LayoutStyle, MeasuredSize, SizeProposal};
///
/// let style = LayoutStyle::min(4, 1).preferred_width(12).max_width(20);
/// assert_eq!(style.resolve_fallback_size(SizeProposal::at_most(10, 5)),
///            MeasuredSize::new(10, 1));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LayoutStyle {
    /// Minimum width in terminal columns; defaults to zero.
    pub min_width: u16,
    /// Maximum width in terminal columns, or `None` for no upper bound.
    pub max_width: Option<u16>,
    /// Preferred width in columns, or `None` to use intrinsic sizing.
    pub preferred_width: Option<u16>,
    /// Minimum height in terminal rows; defaults to zero.
    pub min_height: u16,
    /// Maximum height in terminal rows, or `None` for no upper bound.
    pub max_height: Option<u16>,
    /// Preferred height in rows, or `None` to use intrinsic sizing.
    pub preferred_height: Option<u16>,
    /// Relative share of extra main-axis space; defaults to `0.0` (no growth).
    pub flex_grow: f32,
    /// Relative shrink factor when space is scarce; defaults to `1.0`.
    pub flex_shrink: f32,
    /// Horizontal overflow preference; defaults to [`Overflow::Visible`].
    pub overflow_x: Overflow,
    /// Vertical overflow preference; defaults to [`Overflow::Visible`].
    pub overflow_y: Overflow,
    /// Preferred width-to-height ratio, or `None`. Currently not applied by the Taffy bridge.
    pub aspect_ratio: Option<f32>,
}

impl LayoutStyle {
    /// Creates a content-sized style with no growth or upper size bounds.
    pub fn new() -> Self {
        Self::from_constraints(Constraints::content())
    }

    /// Creates a style with both axes fixed to the given cell counts.
    pub fn fixed(width: u16, height: u16) -> Self {
        Self::from_constraints(Constraints::fixed(width, height))
    }

    /// Creates an unbounded style with zero minimums and a growth factor of `1.0`.
    pub fn fill() -> Self {
        Self::from_constraints(Constraints::fill())
    }

    /// Creates a style with minimum cell dimensions and no growth or upper bounds.
    pub fn min(width: u16, height: u16) -> Self {
        Self::from_constraints(Constraints::min(width, height))
    }

    /// Converts basic constraints into a layout style.
    ///
    /// An axis with equal minimum and maximum becomes a preferred fixed size.
    /// The flex value becomes `flex_grow` (zero for `None`); shrink defaults to
    /// `1.0`, overflow to visible, and aspect ratio to `None`.
    pub fn from_constraints(constraints: Constraints) -> Self {
        let preferred_width = constraints
            .max_width
            .filter(|max| *max == constraints.min_width);
        let preferred_height = constraints
            .max_height
            .filter(|max| *max == constraints.min_height);

        Self {
            min_width: constraints.min_width,
            max_width: constraints.max_width,
            preferred_width,
            min_height: constraints.min_height,
            max_height: constraints.max_height,
            preferred_height,
            flex_grow: constraints.flex.unwrap_or(0.0),
            flex_shrink: 1.0,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            aspect_ratio: None,
        }
    }

    /// Resolves a size without measuring content.
    ///
    /// Unbounded axes use the preferred size or minimum. `AtMost` caps that
    /// fallback and `Exact` replaces it. The result is then clamped to this
    /// style, so style limits can override the proposal.
    pub fn resolve_fallback_size(self, proposal: SizeProposal) -> MeasuredSize {
        let width = resolve_axis(
            proposal.width,
            self.preferred_width.unwrap_or(self.min_width),
        );
        let height = resolve_axis(
            proposal.height,
            self.preferred_height.unwrap_or(self.min_height),
        );

        self.clamp_size(MeasuredSize { width, height })
    }

    /// Applies minimum and maximum bounds to each axis.
    ///
    /// The minimum is applied first, then the maximum. If a maximum is below
    /// the minimum, the maximum wins. Other style fields are not consulted.
    pub fn clamp_size(self, size: MeasuredSize) -> MeasuredSize {
        let width = clamp_axis(size.width, self.min_width, self.max_width);
        let height = clamp_axis(size.height, self.min_height, self.max_height);
        MeasuredSize { width, height }
    }

    /// Sets the preferred width in terminal columns.
    pub fn preferred_width(mut self, width: u16) -> Self {
        self.preferred_width = Some(width);
        self
    }

    /// Sets the preferred height in terminal rows.
    pub fn preferred_height(mut self, height: u16) -> Self {
        self.preferred_height = Some(height);
        self
    }

    /// Sets the maximum width in terminal columns.
    pub fn max_width(mut self, width: u16) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Sets the maximum height in terminal rows.
    pub fn max_height(mut self, height: u16) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Sets the main-axis growth factor; does not change the shrink factor.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Sets the horizontal overflow preference.
    pub fn overflow_x(mut self, overflow: Overflow) -> Self {
        self.overflow_x = overflow;
        self
    }

    /// Sets the vertical overflow preference.
    pub fn overflow_y(mut self, overflow: Overflow) -> Self {
        self.overflow_y = overflow;
        self
    }
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_axis(limit: AxisLimit, fallback: u16) -> u16 {
    match limit {
        AxisLimit::Unbounded => fallback,
        AxisLimit::AtMost(max) => fallback.min(max),
        AxisLimit::Exact(value) => value,
    }
}

fn clamp_axis(value: u16, min: u16, max: Option<u16>) -> u16 {
    let value = value.max(min);
    max.map(|max| value.min(max)).unwrap_or(value)
}

/// Size constraints for layout
#[derive(Debug, Clone, Copy)]
pub struct Constraints {
    /// Minimum width in terminal columns; defaults to zero.
    pub min_width: u16,
    /// Maximum width in terminal columns, or `None` for no upper bound.
    pub max_width: Option<u16>,
    /// Minimum height in terminal rows; defaults to zero.
    pub min_height: u16,
    /// Maximum height in terminal rows, or `None` for no upper bound.
    pub max_height: Option<u16>,
    /// Optional main-axis growth factor; `None` means no growth.
    pub flex: Option<f32>,
}

impl Constraints {
    /// Create fixed-size constraints
    pub fn fixed(width: u16, height: u16) -> Self {
        Self {
            min_width: width,
            max_width: Some(width),
            min_height: height,
            max_height: Some(height),
            flex: None,
        }
    }

    /// Create flexible constraints that fill available space
    pub fn fill() -> Self {
        Self {
            min_width: 0,
            max_width: None,
            min_height: 0,
            max_height: None,
            flex: Some(1.0),
        }
    }

    /// Create minimum-size constraints
    pub fn min(width: u16, height: u16) -> Self {
        Self {
            min_width: width,
            max_width: None,
            min_height: height,
            max_height: None,
            flex: None,
        }
    }

    /// Create content-based constraints (no flex)
    pub fn content() -> Self {
        Self {
            min_width: 0,
            max_width: None,
            min_height: 0,
            max_height: None,
            flex: None,
        }
    }
}
