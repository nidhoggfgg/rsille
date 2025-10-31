//! CSS-style syntax support for widget styling

use super::{BorderStyle, Color, Padding, Style};
use crate::layout::Constraints;
use std::collections::HashMap;

/// CSS parsing error
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

/// Parse CSS-style string into a map of properties
/// Supports formats:
/// - "color: red; background-color: blue"
/// - "font-weight: bold; padding: 1"
pub fn parse_css(css: &str) -> Result<HashMap<String, String>, CssError> {
    let mut properties = HashMap::new();

    // Split by semicolon and process each property
    for declaration in css.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }

        // Split property and value by colon
        let parts: Vec<&str> = declaration.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(CssError::ParseError(format!(
                "Invalid declaration: '{}'",
                declaration
            )));
        }

        let property = parts[0].trim().to_lowercase();
        let value = parts[1].trim().to_string();

        properties.insert(property, value);
    }

    Ok(properties)
}

/// Parse a color value from CSS
/// Supports:
/// - Named colors: "red", "blue", "green", etc.
/// - RGB: "rgb(255, 0, 0)" or "#ff0000"
/// - Indexed: "idx(123)"
pub fn parse_color(value: &str) -> Result<Color, CssError> {
    let value = value.trim().to_lowercase();

    // Named colors
    match value.as_str() {
        "black" => return Ok(Color::Black),
        "red" => return Ok(Color::Red),
        "green" => return Ok(Color::Green),
        "yellow" => return Ok(Color::Yellow),
        "blue" => return Ok(Color::Blue),
        "magenta" => return Ok(Color::Magenta),
        "cyan" => return Ok(Color::Cyan),
        "white" => return Ok(Color::White),
        _ => {}
    }

    // Hex color: #RRGGBB or #RGB
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| CssError::InvalidValue {
                    property: "color".to_string(),
                    value: value.clone(),
                })?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| CssError::InvalidValue {
                    property: "color".to_string(),
                    value: value.clone(),
                })?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| CssError::InvalidValue {
                    property: "color".to_string(),
                    value: value.clone(),
                })?;
            return Ok(Color::Rgb(r, g, b));
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16)
                .map_err(|_| CssError::InvalidValue {
                    property: "color".to_string(),
                    value: value.clone(),
                })?;
            let g = u8::from_str_radix(&hex[1..2], 16)
                .map_err(|_| CssError::InvalidValue {
                    property: "color".to_string(),
                    value: value.clone(),
                })?;
            let b = u8::from_str_radix(&hex[2..3], 16)
                .map_err(|_| CssError::InvalidValue {
                    property: "color".to_string(),
                    value: value.clone(),
                })?;
            return Ok(Color::Rgb(r * 17, g * 17, b * 17));
        }
    }

    // RGB function: rgb(r, g, b)
    if let Some(rgb) = value.strip_prefix("rgb(") {
        if let Some(rgb) = rgb.strip_suffix(')') {
            let parts: Vec<&str> = rgb.split(',').map(|s| s.trim()).collect();
            if parts.len() == 3 {
                let r = parts[0].parse::<u8>().map_err(|_| CssError::InvalidValue {
                    property: "color".to_string(),
                    value: value.clone(),
                })?;
                let g = parts[1].parse::<u8>().map_err(|_| CssError::InvalidValue {
                    property: "color".to_string(),
                    value: value.clone(),
                })?;
                let b = parts[2].parse::<u8>().map_err(|_| CssError::InvalidValue {
                    property: "color".to_string(),
                    value: value.clone(),
                })?;
                return Ok(Color::Rgb(r, g, b));
            }
        }
    }

    // Indexed color: idx(n)
    if let Some(idx) = value.strip_prefix("idx(") {
        if let Some(idx) = idx.strip_suffix(')') {
            let n = idx.parse::<u8>().map_err(|_| CssError::InvalidValue {
                property: "color".to_string(),
                value: value.clone(),
            })?;
            return Ok(Color::Indexed(n));
        }
    }

    Err(CssError::InvalidValue {
        property: "color".to_string(),
        value,
    })
}

/// Parse padding value
/// Supports:
/// - Single value: "1" -> uniform padding
/// - Four values: "1 2 3 4" -> top right bottom left
pub fn parse_padding(value: &str) -> Result<Padding, CssError> {
    let parts: Vec<&str> = value.split_whitespace().collect();

    match parts.len() {
        1 => {
            let uniform = parts[0].parse::<u16>().map_err(|_| CssError::InvalidValue {
                property: "padding".to_string(),
                value: value.to_string(),
            })?;
            Ok(Padding::uniform(uniform))
        }
        4 => {
            let top = parts[0].parse::<u16>().map_err(|_| CssError::InvalidValue {
                property: "padding".to_string(),
                value: value.to_string(),
            })?;
            let right = parts[1].parse::<u16>().map_err(|_| CssError::InvalidValue {
                property: "padding".to_string(),
                value: value.to_string(),
            })?;
            let bottom = parts[2].parse::<u16>().map_err(|_| CssError::InvalidValue {
                property: "padding".to_string(),
                value: value.to_string(),
            })?;
            let left = parts[3].parse::<u16>().map_err(|_| CssError::InvalidValue {
                property: "padding".to_string(),
                value: value.to_string(),
            })?;
            Ok(Padding::new(top, right, bottom, left))
        }
        _ => Err(CssError::InvalidValue {
            property: "padding".to_string(),
            value: value.to_string(),
        }),
    }
}

/// Parse border style
/// Supports: "single", "rounded", "double", "thick", "none"
pub fn parse_border(value: &str) -> Result<BorderStyle, CssError> {
    match value.trim().to_lowercase().as_str() {
        "single" | "solid" => Ok(BorderStyle::Single),
        "rounded" => Ok(BorderStyle::Rounded),
        "double" => Ok(BorderStyle::Double),
        "thick" => Ok(BorderStyle::Thick),
        "none" => Ok(BorderStyle::None),
        _ => Err(CssError::InvalidValue {
            property: "border".to_string(),
            value: value.to_string(),
        }),
    }
}

impl Style {
    /// Create a style from CSS-like syntax
    ///
    /// # Supported properties
    /// - `color`: Foreground color (named, #RGB, #RRGGBB, rgb(r,g,b), idx(n))
    /// - `background-color`: Background color (same formats as color)
    /// - `font-weight`: "bold" to enable bold
    /// - `font-style`: "italic" to enable italic
    /// - `text-decoration`: "underline" to enable underline
    /// - `border`: Border style ("solid", "rounded", "double", "thick")
    /// - `padding`: Padding (single value or "top right bottom left")
    ///
    /// # Example
    /// ```
    /// use tui::prelude::*;
    /// let style = Style::from_css("color: red; background-color: #282c34; font-weight: bold");
    /// ```
    pub fn from_css(css: &str) -> Result<Self, CssError> {
        let properties = parse_css(css)?;
        let mut style = Style::default();

        for (property, value) in properties {
            match property.as_str() {
                "color" => {
                    style.fg_color = Some(parse_color(&value)?);
                }
                "background-color" | "background" => {
                    style.bg_color = Some(parse_color(&value)?);
                }
                "font-weight" => {
                    if value.to_lowercase() == "bold" {
                        style.modifiers = style.modifiers.with_bold();
                    }
                }
                "font-style" => {
                    if value.to_lowercase() == "italic" {
                        style.modifiers = style.modifiers.with_italic();
                    }
                }
                "text-decoration" => {
                    if value.to_lowercase() == "underline" {
                        style.modifiers = style.modifiers.with_underlined();
                    }
                }
                "border" => {
                    style.border = Some(parse_border(&value)?);
                }
                "padding" => {
                    style.padding = parse_padding(&value)?;
                }
                _ => {
                    // Ignore unknown properties for forward compatibility
                }
            }
        }

        Ok(style)
    }

    /// Builder method to apply CSS styling
    pub fn css(css: &str) -> Result<Self, CssError> {
        Self::from_css(css)
    }
}

impl Constraints {
    /// Create constraints from CSS-like syntax
    ///
    /// # Supported properties
    /// - `width`: Fixed width in cells
    /// - `min-width`: Minimum width
    /// - `max-width`: Maximum width
    /// - `height`: Fixed height in cells
    /// - `min-height`: Minimum height
    /// - `max-height`: Maximum height
    /// - `flex`: Flex grow factor
    ///
    /// # Example
    /// ```
    /// use tui::prelude::*;
    /// let constraints = Constraints::from_css("width: 20; height: 10; flex: 1");
    /// ```
    pub fn from_css(css: &str) -> Result<Self, CssError> {
        let properties = parse_css(css)?;
        let mut constraints = Constraints::content();

        for (property, value) in properties {
            match property.as_str() {
                "width" => {
                    let width = value.parse::<u16>().map_err(|_| CssError::InvalidValue {
                        property: "width".to_string(),
                        value: value.clone(),
                    })?;
                    constraints.min_width = width;
                    constraints.max_width = Some(width);
                }
                "min-width" => {
                    let min_width = value.parse::<u16>().map_err(|_| CssError::InvalidValue {
                        property: "min-width".to_string(),
                        value: value.clone(),
                    })?;
                    constraints.min_width = min_width;
                }
                "max-width" => {
                    let max_width = value.parse::<u16>().map_err(|_| CssError::InvalidValue {
                        property: "max-width".to_string(),
                        value: value.clone(),
                    })?;
                    constraints.max_width = Some(max_width);
                }
                "height" => {
                    let height = value.parse::<u16>().map_err(|_| CssError::InvalidValue {
                        property: "height".to_string(),
                        value: value.clone(),
                    })?;
                    constraints.min_height = height;
                    constraints.max_height = Some(height);
                }
                "min-height" => {
                    let min_height = value.parse::<u16>().map_err(|_| CssError::InvalidValue {
                        property: "min-height".to_string(),
                        value: value.clone(),
                    })?;
                    constraints.min_height = min_height;
                }
                "max-height" => {
                    let max_height = value.parse::<u16>().map_err(|_| CssError::InvalidValue {
                        property: "max-height".to_string(),
                        value: value.clone(),
                    })?;
                    constraints.max_height = Some(max_height);
                }
                "flex" => {
                    let flex = value.parse::<f32>().map_err(|_| CssError::InvalidValue {
                        property: "flex".to_string(),
                        value: value.clone(),
                    })?;
                    constraints.flex = Some(flex);
                }
                _ => {
                    // Ignore unknown properties for forward compatibility
                }
            }
        }

        Ok(constraints)
    }

    /// Builder method to apply CSS constraints
    pub fn css(css: &str) -> Result<Self, CssError> {
        Self::from_css(css)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_named_color() {
        assert_eq!(parse_color("red").unwrap(), Color::Red);
        assert_eq!(parse_color("blue").unwrap(), Color::Blue);
        assert_eq!(parse_color("RED").unwrap(), Color::Red);
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_color("#ff0000").unwrap(), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("#f00").unwrap(), Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_parse_rgb_color() {
        assert_eq!(parse_color("rgb(255, 0, 0)").unwrap(), Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_parse_indexed_color() {
        assert_eq!(parse_color("idx(123)").unwrap(), Color::Indexed(123));
    }

    #[test]
    fn test_parse_padding_uniform() {
        let padding = parse_padding("2").unwrap();
        assert_eq!(padding, Padding::uniform(2));
    }

    #[test]
    fn test_parse_padding_sides() {
        let padding = parse_padding("1 2 3 4").unwrap();
        assert_eq!(padding, Padding::new(1, 2, 3, 4));
    }

    #[test]
    fn test_style_from_css() {
        let style = Style::from_css("color: red; background-color: blue; font-weight: bold").unwrap();
        assert_eq!(style.fg_color, Some(Color::Red));
        assert_eq!(style.bg_color, Some(Color::Blue));
        assert!(style.modifiers.contains_bold());
    }

    #[test]
    fn test_constraints_from_css() {
        let constraints = Constraints::from_css("width: 20; height: 10; flex: 1").unwrap();
        assert_eq!(constraints.min_width, 20);
        assert_eq!(constraints.max_width, Some(20));
        assert_eq!(constraints.min_height, 10);
        assert_eq!(constraints.max_height, Some(10));
        assert_eq!(constraints.flex, Some(1.0));
    }
}
