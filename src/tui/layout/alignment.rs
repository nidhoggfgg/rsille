//! Shared helpers for aligning content inside an allocated area.

/// Horizontal placement of content inside available terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl HorizontalAlign {
    /// Returns the leading cell offset for `content` inside `available`.
    pub fn offset(self, available: u16, content: u16) -> u16 {
        if content >= available {
            return 0;
        }

        let extra = available - content;
        match self {
            Self::Left => 0,
            Self::Center => extra / 2,
            Self::Right => extra,
        }
    }
}

/// Vertical placement of content inside available terminal rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

impl VerticalAlign {
    /// Returns the leading row offset for `content` inside `available`.
    pub fn offset(self, available: u16, content: u16) -> u16 {
        if content >= available {
            return 0;
        }

        let extra = available - content;
        match self {
            Self::Top => 0,
            Self::Middle => extra / 2,
            Self::Bottom => extra,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_offsets_fit_content_inside_available_width() {
        assert_eq!(HorizontalAlign::Left.offset(10, 4), 0);
        assert_eq!(HorizontalAlign::Center.offset(10, 4), 3);
        assert_eq!(HorizontalAlign::Right.offset(10, 4), 6);
    }

    #[test]
    fn offsets_saturate_when_content_is_larger_than_available_space() {
        assert_eq!(HorizontalAlign::Center.offset(4, 10), 0);
        assert_eq!(VerticalAlign::Middle.offset(2, 3), 0);
    }
}
