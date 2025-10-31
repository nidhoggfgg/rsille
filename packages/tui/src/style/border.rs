//! Border styles using Unicode box-drawing characters

/// Border style variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    None,
    /// ┌─┐│└─┘
    Single,
    /// ╔═╗║╚═╝
    Double,
    /// ╭─╮│╰─╯
    Rounded,
    /// ┏━┓┃┗━┛
    Thick,
}

impl BorderStyle {
    /// Get the box-drawing characters for this style
    pub fn chars(&self) -> BorderChars {
        match self {
            BorderStyle::None => BorderChars::none(),
            BorderStyle::Single => BorderChars {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
                vertical: '│',
                horizontal: '─',
            },
            BorderStyle::Double => BorderChars {
                top_left: '╔',
                top_right: '╗',
                bottom_left: '╚',
                bottom_right: '╝',
                vertical: '║',
                horizontal: '═',
            },
            BorderStyle::Rounded => BorderChars {
                top_left: '╭',
                top_right: '╮',
                bottom_left: '╰',
                bottom_right: '╯',
                vertical: '│',
                horizontal: '─',
            },
            BorderStyle::Thick => BorderChars {
                top_left: '┏',
                top_right: '┓',
                bottom_left: '┗',
                bottom_right: '┛',
                vertical: '┃',
                horizontal: '━',
            },
        }
    }
}

/// Box-drawing character set
#[derive(Debug, Clone, Copy)]
pub struct BorderChars {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub vertical: char,
    pub horizontal: char,
}

impl BorderChars {
    fn none() -> Self {
        Self {
            top_left: ' ',
            top_right: ' ',
            bottom_left: ' ',
            bottom_right: ' ',
            vertical: ' ',
            horizontal: ' ',
        }
    }
}
