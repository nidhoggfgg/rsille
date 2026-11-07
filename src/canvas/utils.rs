//! Rounding and coordinate conversion helpers for the 2×4 Braille grid.
//!
//! These helpers always use Braille cell dimensions, regardless of a canvas's
//! encoding. For other encodings, use [`Encoding::samples_per_cell`](super::Encoding::samples_per_cell)
//! to obtain the dimensions. Cell rows, like sample coordinates, grow upwards.

/// Rounds a numeric value to the nearest `i32`, with ties away from zero.
///
/// Uses Rust's floating-point-to-integer cast: NaN becomes zero and values
/// outside the `i32` range (including infinities) saturate at its endpoints.
/// Unlike canvas point operations, this helper does not reject these values.
///
/// ```
/// use rsille::canvas::utils::round;
///
/// assert_eq!(round(1.5), 2);
/// assert_eq!(round(-1.5), -2);
/// assert_eq!(round(f64::NAN), 0);
/// ```
#[inline]
pub fn round<T>(v: T) -> i32
where
    T: Into<f64>,
{
    v.into().round() as i32
}

/// Rounds an `f64` with the same tie-breaking and saturation rules as [`round`].
#[inline]
pub fn round_f64(v: f64) -> i32 {
    v.round() as i32
}

/// Returns the Braille cell `(column, row)` containing sample `(x, y)`.
///
/// Rounds each coordinate with [`round`], then groups samples into 2×4 cells.
/// Negative coordinates map to cells to the left of or below the origin.
///
/// # Panics
///
/// With overflow checks enabled, panics if `x` rounds or saturates to
/// [`i32::MIN`], because [`get_col`] subtracts one from negative coordinates.
///
/// ```
/// use rsille::canvas::utils::get_pos;
///
/// assert_eq!(get_pos(1, 3), (0, 0));
/// assert_eq!(get_pos(2, 4), (1, 1));
/// assert_eq!(get_pos(-1, -1), (-1, -1));
/// ```
pub fn get_pos<T>(x: T, y: T) -> (i32, i32)
where
    T: Into<f64>,
{
    let (x, y) = (round(x), round(y));
    let row = get_row(y);
    let col = get_col(x);
    (col, row)
}

/// Returns the Cartesian Braille cell row containing sample coordinate `y`.
///
/// Equivalent to floor division by 4: samples `0..=3` are in row 0 and
/// samples `-4..=-1` are in row -1.
pub fn get_row(y: i32) -> i32 {
    if y < 0 {
        (y + 1) / 4 - 1
    } else {
        y / 4
    }
}

/// Returns the Braille cell column containing sample coordinate `x`.
///
/// For `x > i32::MIN`, this is floor division by 2: samples `0..=1` are in
/// column 0 and samples `-2..=-1` are in column -1. The current arithmetic
/// overflows for `i32::MIN`; with overflow checks disabled it wraps and returns
/// an incorrect positive column.
///
/// # Panics
///
/// Panics for [`i32::MIN`] when overflow checks are enabled.
pub fn get_col(x: i32) -> i32 {
    if x < 0 {
        (x - 1) / 2
    } else {
        x / 2
    }
}
