//! Border styles using Unicode box-drawing characters.

/// A box-drawing style for widget borders and layout separators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// No visible border; all characters returned by [`Self::chars`] are spaces.
    None,
    /// Thin single lines: `┌─┐│└─┘`.
    Single,
    /// Double lines: `╔═╗║╚═╝`.
    Double,
    /// Thin lines with rounded corners: `╭─╮│╰─╯`.
    Rounded,
    /// Heavy lines: `┏━┓┃┗━┛`.
    Thick,
}

impl BorderStyle {
    /// Returns the corners, lines, and junctions for this style.
    ///
    /// Rounded borders use single-line junctions. [`Self::None`] returns spaces
    /// for every character.
    pub fn chars(&self) -> BorderChars {
        match self {
            BorderStyle::None => BorderChars::none(),
            BorderStyle::Single => BorderChars {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
                horizontal: '─',
                vertical: '│',
                top_junction: '┬',
                bottom_junction: '┴',
                left_junction: '├',
                right_junction: '┤',
                cross: '┼',
            },
            BorderStyle::Double => BorderChars {
                top_left: '╔',
                top_right: '╗',
                bottom_left: '╚',
                bottom_right: '╝',
                horizontal: '═',
                vertical: '║',
                top_junction: '╦',
                bottom_junction: '╩',
                left_junction: '╠',
                right_junction: '╣',
                cross: '╬',
            },
            BorderStyle::Rounded => BorderChars {
                top_left: '╭',
                top_right: '╮',
                bottom_left: '╰',
                bottom_right: '╯',
                horizontal: '─',
                vertical: '│',
                // Rounded corners don't have junctions, fall back to single style
                top_junction: '┬',
                bottom_junction: '┴',
                left_junction: '├',
                right_junction: '┤',
                cross: '┼',
            },
            BorderStyle::Thick => BorderChars {
                top_left: '┏',
                top_right: '┓',
                bottom_left: '┗',
                bottom_right: '┛',
                horizontal: '━',
                vertical: '┃',
                top_junction: '┳',
                bottom_junction: '┻',
                left_junction: '┣',
                right_junction: '┫',
                cross: '╋',
            },
        }
    }
}

/// Box-drawing character set for borders and layouts
///
/// Provides a complete set of Unicode box-drawing characters for creating
/// borders, tables, separators, and complex layouts.
///
/// # Examples
/// ```
/// use rsille::tui::style::{BorderStyle, BorderChars};
///
/// let chars = BorderStyle::Single.chars();
/// // Use basic characters for simple borders
/// println!("{}{}{}", chars.top_left, chars.horizontal, chars.top_right);
///
/// // Use junction characters for tables
/// println!("{}{}{}", chars.left_junction, chars.horizontal, chars.right_junction);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BorderChars {
    // Corner characters
    /// Upper-left corner, such as `┌`.
    pub top_left: char,
    /// Upper-right corner, such as `┐`.
    pub top_right: char,
    /// Lower-left corner, such as `└`.
    pub bottom_left: char,
    /// Lower-right corner, such as `┘`.
    pub bottom_right: char,

    // Line characters
    /// Horizontal line segment, such as `─`.
    pub horizontal: char,
    /// Vertical line segment, such as `│`.
    pub vertical: char,

    // T-junction characters (for connecting borders)
    /// T-junction pointing down (e.g., ┬ for single, ╦ for double)
    pub top_junction: char,
    /// T-junction pointing up (e.g., ┴ for single, ╩ for double)
    pub bottom_junction: char,
    /// T-junction pointing right (e.g., ├ for single, ╠ for double)
    pub left_junction: char,
    /// T-junction pointing left (e.g., ┤ for single, ╣ for double)
    pub right_junction: char,

    // Cross character (for table intersections)
    /// Cross/intersection character (e.g., ┼ for single, ╬ for double)
    pub cross: char,
}

impl BorderChars {
    /// Create an empty border character set (all spaces)
    fn none() -> Self {
        Self {
            top_left: ' ',
            top_right: ' ',
            bottom_left: ' ',
            bottom_right: ' ',
            horizontal: ' ',
            vertical: ' ',
            top_junction: ' ',
            bottom_junction: ' ',
            left_junction: ' ',
            right_junction: ' ',
            cross: ' ',
        }
    }
}
