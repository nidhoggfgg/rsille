//! Size constraint types for layout calculation

/// Per-axis size proposal supplied by a parent during measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisLimit {
    Unbounded,
    AtMost(u16),
    Exact(u16),
}

impl AxisLimit {
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
    pub width: AxisLimit,
    pub height: AxisLimit,
}

impl SizeProposal {
    pub const UNBOUNDED: Self = Self {
        width: AxisLimit::Unbounded,
        height: AxisLimit::Unbounded,
    };

    pub fn exact(width: u16, height: u16) -> Self {
        Self {
            width: AxisLimit::Exact(width),
            height: AxisLimit::Exact(height),
        }
    }

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
    pub width: u16,
    pub height: u16,
}

impl MeasuredSize {
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

/// Overflow preference for a widget axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Visible,
    Clip,
    Scroll,
}

/// Rich layout intent for a widget.
///
/// This is the measurement-era replacement for plain [`Constraints`]. The
/// old constraints remain the source-compatible API and are converted into
/// this richer style by default.
#[derive(Debug, Clone, Copy)]
pub struct LayoutStyle {
    pub min_width: u16,
    pub max_width: Option<u16>,
    pub preferred_width: Option<u16>,
    pub min_height: u16,
    pub max_height: Option<u16>,
    pub preferred_height: Option<u16>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub aspect_ratio: Option<f32>,
}

impl LayoutStyle {
    pub fn new() -> Self {
        Self::from_constraints(Constraints::content())
    }

    pub fn fixed(width: u16, height: u16) -> Self {
        Self::from_constraints(Constraints::fixed(width, height))
    }

    pub fn fill() -> Self {
        Self::from_constraints(Constraints::fill())
    }

    pub fn min(width: u16, height: u16) -> Self {
        Self::from_constraints(Constraints::min(width, height))
    }

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

    pub fn clamp_size(self, size: MeasuredSize) -> MeasuredSize {
        let width = clamp_axis(size.width, self.min_width, self.max_width);
        let height = clamp_axis(size.height, self.min_height, self.max_height);
        MeasuredSize { width, height }
    }

    pub fn preferred_width(mut self, width: u16) -> Self {
        self.preferred_width = Some(width);
        self
    }

    pub fn preferred_height(mut self, height: u16) -> Self {
        self.preferred_height = Some(height);
        self
    }

    pub fn max_width(mut self, width: u16) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn max_height(mut self, height: u16) -> Self {
        self.max_height = Some(height);
        self
    }

    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    pub fn overflow_x(mut self, overflow: Overflow) -> Self {
        self.overflow_x = overflow;
        self
    }

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
    pub min_width: u16,
    pub max_width: Option<u16>,
    pub min_height: u16,
    pub max_height: Option<u16>,
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
