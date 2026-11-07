//! Errors from CSS-like style and layout declarations.

/// An error parsing a CSS-like declaration or its value.
///
/// Returned by [`super::Style::from_css`] and
/// [`Constraints::from_css`](crate::tui::layout::Constraints::from_css).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssError {
    /// An unsupported property name.
    ///
    /// The current style and constraint parsers ignore unknown properties and
    /// do not emit this variant.
    InvalidProperty(String),
    /// A recognized property has a value that cannot be parsed.
    InvalidValue {
        /// The property whose value could not be parsed.
        property: String,
        /// The rejected value, possibly normalized by the parser.
        value: String,
    },
    /// A malformed declaration, with a description of the syntax error.
    ParseError(String),
}

impl std::fmt::Display for CssError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CssError::InvalidProperty(prop) => write!(f, "Invalid CSS property: {}", prop),
            CssError::InvalidValue { property, value } => {
                write!(f, "Invalid value '{}' for property '{}'", value, property)
            }
            CssError::ParseError(msg) => write!(f, "CSS parse error: {}", msg),
        }
    }
}

impl std::error::Error for CssError {}
