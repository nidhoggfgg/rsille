//! Padding in terminal cells for layout spacing.

/// Interior spacing around widget content, measured in terminal cells.
///
/// Top and bottom values count rows; left and right values count columns.
/// Configure padding on the widget separately from [`super::Style`].
///
/// # Examples
///
/// ```
/// use rsille::tui::style::Padding;
///
/// let padding = Padding::new(1, 2, 1, 2);
/// assert_eq!(padding.horizontal_total(), 4);
/// assert_eq!(padding.vertical_total(), 2);
/// assert_eq!(Padding::horizontal(2), Padding::new(0, 2, 0, 2));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Padding {
    /// Rows above the content.
    pub top: u16,
    /// Columns to the right of the content.
    pub right: u16,
    /// Rows below the content.
    pub bottom: u16,
    /// Columns to the left of the content.
    pub left: u16,
}

impl Padding {
    /// Zero padding on every side.
    pub const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// Creates padding with the same cell count on all four sides.
    pub fn uniform(value: u16) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Creates padding in clockwise order: top, right, bottom, left.
    pub fn new(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Sets left and right padding to `value`, leaving top and bottom at zero.
    pub fn horizontal(value: u16) -> Self {
        Self {
            top: 0,
            right: value,
            bottom: 0,
            left: value,
        }
    }

    /// Sets top and bottom padding to `value`, leaving left and right at zero.
    pub fn vertical(value: u16) -> Self {
        Self {
            top: value,
            right: 0,
            bottom: value,
            left: 0,
        }
    }

    /// Returns the sum of the left and right padding in columns.
    ///
    /// The sum must fit in a `u16`; addition is not saturating.
    pub fn horizontal_total(&self) -> u16 {
        self.left + self.right
    }

    /// Returns the sum of the top and bottom padding in rows.
    ///
    /// The sum must fit in a `u16`; addition is not saturating.
    pub fn vertical_total(&self) -> u16 {
        self.top + self.bottom
    }
}
