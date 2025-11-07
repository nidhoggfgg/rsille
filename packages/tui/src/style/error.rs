#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssError {
    InvalidProperty(String),
    InvalidValue { property: String, value: String },
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
