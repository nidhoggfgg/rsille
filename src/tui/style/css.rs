//! CSS-like declaration parsing for visual styles and layout constraints.

use super::{Color, Style};
use crate::tui::{layout::Constraints, style::CssError};
use std::collections::HashMap;

/// Parses semicolon-separated `property: value` declarations into a map.
///
/// Whitespace is trimmed, property names are lowercased, and values retain their
/// case. Empty declarations are skipped. Repeated properties keep their last
/// value; different properties have no guaranteed iteration order. This step
/// does not validate property names or values.
///
/// # Errors
///
/// Returns [`CssError::ParseError`] for a nonempty declaration without a colon.
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

/// Parses a named, RGB, hexadecimal, or indexed color, ignoring case.
///
/// Accepts the eight named [`Color`] variants, `#RGB`, `#RRGGBB`, `rgb(r, g, b)`,
/// and `idx(n)`. RGB channels and palette indices must fit in a `u8`.
///
/// # Errors
///
/// Returns [`CssError::InvalidValue`] with property `color` if the format or
/// numeric value is invalid, including when parsing a background color.
///
/// # Panics
///
/// Malformed hexadecimal input containing non-ASCII characters may panic if a
/// channel boundary falls inside a UTF-8 character.
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
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| CssError::InvalidValue {
                property: "color".to_string(),
                value: value.clone(),
            })?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| CssError::InvalidValue {
                property: "color".to_string(),
                value: value.clone(),
            })?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| CssError::InvalidValue {
                property: "color".to_string(),
                value: value.clone(),
            })?;
            return Ok(Color::Rgb(r, g, b));
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).map_err(|_| CssError::InvalidValue {
                property: "color".to_string(),
                value: value.clone(),
            })?;
            let g = u8::from_str_radix(&hex[1..2], 16).map_err(|_| CssError::InvalidValue {
                property: "color".to_string(),
                value: value.clone(),
            })?;
            let b = u8::from_str_radix(&hex[2..3], 16).map_err(|_| CssError::InvalidValue {
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

impl Style {
    /// Creates a style from semicolon-separated CSS-like declarations.
    ///
    /// Property names and recognized keyword values are case-insensitive.
    /// Duplicate property names keep the last value. Unknown properties and
    /// unrecognized text attribute values are ignored. Empty input produces an
    /// empty style. This parser accepts declarations only, without selectors or
    /// braces.
    ///
    /// # Supported properties
    ///
    /// - `color`: Foreground color: one of the eight named [`Color`] variants,
    ///   `#RGB`, `#RRGGBB`, `rgb(r, g, b)`, or `idx(n)`.
    /// - `background-color` or `background`: Background color in the same formats.
    /// - `font-weight`: `bold` enables bold.
    /// - `font-style`: `italic` enables italic.
    /// - `text-decoration`: `underline` enables underline.
    ///
    /// Color channels and palette indices range from `0` to `255`. Layout
    /// properties such as `border` and `padding` are ignored; set them on the
    /// widget. Avoid specifying both background aliases, since their application
    /// order is unspecified.
    ///
    /// # Errors
    ///
    /// Returns [`CssError::ParseError`] for declarations without a colon, or
    /// [`CssError::InvalidValue`] for invalid colors. Color errors identify the
    /// property as `color`, including errors in background declarations.
    ///
    /// # Panics
    ///
    /// Malformed hexadecimal colors containing non-ASCII characters may panic
    /// if a channel boundary falls inside a UTF-8 character.
    ///
    /// # Examples
    ///
    /// ```
    /// use rsille::tui::style::{Color, CssError, Style};
    ///
    /// let style = Style::from_css(
    ///     "color: red; background-color: #282c34; font-weight: bold",
    /// )?;
    /// assert_eq!(style.fg_color, Some(Color::Red));
    /// assert_eq!(style.bg_color, Some(Color::Rgb(40, 44, 52)));
    /// assert!(style.modifiers.contains_bold());
    /// # Ok::<(), CssError>(())
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
                _ => {
                    // Ignore unknown properties for forward compatibility
                }
            }
        }

        Ok(style)
    }

    /// Creates a style using [`Self::from_css`].
    ///
    /// Accepts the same declarations and has the same error and panic behavior.
    pub fn css(css: &str) -> Result<Self, CssError> {
        Self::from_css(css)
    }
}

impl Constraints {
    /// Creates layout constraints from semicolon-separated CSS-like declarations.
    ///
    /// Starts with [`Self::content`]. Property names are case-insensitive,
    /// duplicate names keep the last value, and unknown properties are ignored.
    /// Dimensions are unitless `u16` cell counts; suffixes such as `px` or `%`
    /// are not supported. The `flex` value is parsed as an `f32`.
    ///
    /// Distinct properties are applied in an unspecified order. Avoid combining
    /// `width` with `min-width` or `max-width`, or `height` with `min-height` or
    /// `max-height`, since they write the same fields. Bounds and flex values are
    /// not checked for consistency or nonnegativity.
    ///
    /// # Supported properties
    ///
    /// - `width`: Fixed width in cells
    /// - `min-width`: Minimum width
    /// - `max-width`: Maximum width
    /// - `height`: Fixed height in cells
    /// - `min-height`: Minimum height
    /// - `max-height`: Maximum height
    /// - `flex`: Flex grow factor
    ///
    /// # Errors
    ///
    /// Returns [`CssError::ParseError`] for declarations without a colon, or
    /// [`CssError::InvalidValue`] when a recognized value cannot be parsed into
    /// the corresponding numeric type.
    ///
    /// # Examples
    ///
    /// ```
    /// use rsille::tui::{layout::Constraints, style::CssError};
    ///
    /// let constraints = Constraints::from_css("width: 20; height: 10; flex: 1")?;
    /// assert_eq!(constraints.min_width, 20);
    /// assert_eq!(constraints.max_width, Some(20));
    /// assert_eq!(constraints.flex, Some(1.0));
    /// # Ok::<(), CssError>(())
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

    /// Creates constraints using [`Self::from_css`].
    ///
    /// # Errors
    ///
    /// Returns the same syntax and numeric-value errors as [`Self::from_css`].
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
        assert_eq!(
            parse_color("rgb(255, 0, 0)").unwrap(),
            Color::Rgb(255, 0, 0)
        );
    }

    #[test]
    fn test_parse_indexed_color() {
        assert_eq!(parse_color("idx(123)").unwrap(), Color::Indexed(123));
    }

    #[test]
    fn test_style_from_css() {
        let style =
            Style::from_css("color: red; background-color: blue; font-weight: bold").unwrap();
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
