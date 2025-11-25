//! Theme system for global styling

use super::{Color, Style};

/// Semantic color definitions for a theme
#[derive(Debug, Clone)]
pub struct ThemeColors {
    /// Primary accent color
    pub primary: Color,
    /// Secondary accent color
    pub secondary: Color,
    /// Success state color (typically green)
    pub success: Color,
    /// Danger/error state color (typically red)
    pub danger: Color,
    /// Warning state color (typically yellow/orange)
    pub warning: Color,
    /// Info state color (typically blue/cyan)
    pub info: Color,
    /// Main text color
    pub text: Color,
    /// Muted/secondary text color
    pub text_muted: Color,
    /// Main background color
    pub background: Color,
    /// Surface/card background color
    pub surface: Color,
    /// Border color
    pub border: Color,
    /// Focus ring color (for borders and outlines when focused)
    pub focus_ring: Color,
    /// Focus background highlight color (subtle background change when focused)
    pub focus_background: Color,
}

impl ThemeColors {
    /// Create a dark theme color palette
    pub fn dark() -> Self {
        Self {
            primary: Color::Rgb(99, 102, 241),         // Indigo
            secondary: Color::Rgb(139, 92, 246),       // Purple
            success: Color::Rgb(34, 197, 94),          // Green
            danger: Color::Rgb(239, 68, 68),           // Red
            warning: Color::Rgb(251, 146, 60),         // Orange
            info: Color::Rgb(59, 130, 246),            // Blue
            text: Color::Rgb(229, 229, 231),           // Zinc-200
            text_muted: Color::Rgb(161, 161, 170),     // Zinc-400
            background: Color::Rgb(24, 24, 27),        // Zinc-900
            surface: Color::Rgb(39, 39, 42),           // Zinc-800
            border: Color::Rgb(63, 63, 70),            // Zinc-700
            focus_ring: Color::Rgb(129, 140, 248),     // Lighter indigo for focus (Indigo-400)
            focus_background: Color::Rgb(49, 46, 129), // Dark indigo background (Indigo-950)
        }
    }

    /// Create a light theme color palette
    pub fn light() -> Self {
        Self {
            primary: Color::Rgb(79, 70, 229),            // Indigo
            secondary: Color::Rgb(124, 58, 237),         // Purple
            success: Color::Rgb(22, 163, 74),            // Green
            danger: Color::Rgb(220, 38, 38),             // Red
            warning: Color::Rgb(234, 88, 12),            // Orange
            info: Color::Rgb(37, 99, 235),               // Blue
            text: Color::Rgb(24, 24, 27),                // Zinc-900
            text_muted: Color::Rgb(113, 113, 122),       // Zinc-500
            background: Color::Rgb(250, 250, 250),       // Zinc-50
            surface: Color::Rgb(255, 255, 255),          // White
            border: Color::Rgb(212, 212, 216),           // Zinc-300
            focus_ring: Color::Rgb(67, 56, 202),         // Darker indigo for focus (Indigo-700)
            focus_background: Color::Rgb(224, 231, 255), // Light indigo background (Indigo-100)
        }
    }
}

/// Semantic style roles for theming
///
/// This structure provides semantic styling that can be used by any widget,
/// including user-defined custom widgets. Instead of hardcoding styles for
/// specific components, we provide semantic roles based on the purpose and
/// context of UI elements.
#[derive(Debug, Clone)]
pub struct ThemeStyles {
    // === Action Styles ===
    /// Primary action style (e.g., primary buttons, key actions)
    pub primary_action: Style,
    /// Primary action hover state
    pub primary_action_hover: Style,
    /// Primary action focused state
    pub primary_action_focused: Style,
    /// Secondary action style (e.g., secondary buttons)
    pub secondary_action: Style,
    /// Secondary action hover state
    pub secondary_action_hover: Style,
    /// Secondary action focused state
    pub secondary_action_focused: Style,

    // === Interactive Element Styles ===
    /// Interactive element style (e.g., inputs, checkboxes, sliders)
    pub interactive: Style,
    /// Interactive element focused state
    pub interactive_focused: Style,
    /// Interactive element disabled state
    pub interactive_disabled: Style,

    // === Text Styles ===
    /// Regular text style (e.g., labels, paragraphs)
    pub text: Style,
    /// Muted/secondary text style
    pub text_muted: Style,
    /// Heading text style
    pub text_heading: Style,

    // === Container Styles ===
    /// Surface style (e.g., container backgrounds)
    pub surface: Style,
    /// Elevated surface style (e.g., modals, popups, cards)
    pub surface_elevated: Style,

    // === State Styles ===
    /// Selected/highlighted state
    pub selected: Style,
    /// Hover state (generic)
    pub hover: Style,
    /// Disabled state (generic)
    pub disabled: Style,
}

impl ThemeStyles {
    /// Create semantic styles for dark theme
    pub fn dark(colors: &ThemeColors) -> Self {
        Self {
            // Action styles
            primary_action: Style::default().fg(colors.text).bg(colors.primary),
            primary_action_hover: Style::default().fg(colors.text).bg(colors.primary).bold(),
            primary_action_focused: Style::default().fg(colors.text).bg(colors.primary).bold(), // Make focused state visually distinct
            secondary_action: Style::default().fg(colors.text).bg(colors.secondary),
            secondary_action_hover: Style::default().fg(colors.text).bg(colors.secondary).bold(),
            secondary_action_focused: Style::default().fg(colors.text).bg(colors.secondary).bold(), // Make focused state visually distinct

            // Interactive element styles
            interactive: Style::default().fg(colors.text).bg(colors.surface),
            interactive_focused: Style::default().fg(colors.text).bg(colors.surface).bold(), // Make focused state visually distinct
            interactive_disabled: Style::default().fg(colors.text_muted).bg(colors.surface),

            // Text styles
            text: Style::default().fg(colors.text),
            text_muted: Style::default().fg(colors.text_muted),
            text_heading: Style::default().fg(colors.text).bold(),

            // Container styles
            surface: Style::default().bg(colors.background).fg(colors.text),
            surface_elevated: Style::default().bg(colors.surface).fg(colors.text),

            // State styles
            selected: Style::default().fg(colors.text).bg(colors.primary),
            hover: Style::default().fg(colors.text).bg(colors.primary).bold(),
            disabled: Style::default().fg(colors.text_muted),
        }
    }

    /// Create semantic styles for light theme
    pub fn light(colors: &ThemeColors) -> Self {
        Self {
            // Action styles
            primary_action: Style::default().fg(Color::White).bg(colors.primary),
            primary_action_hover: Style::default().fg(Color::White).bg(colors.primary).bold(),
            primary_action_focused: Style::default().fg(Color::White).bg(colors.primary).bold(), // Make focused state visually distinct
            secondary_action: Style::default().fg(Color::White).bg(colors.secondary),
            secondary_action_hover: Style::default()
                .fg(Color::White)
                .bg(colors.secondary)
                .bold(),
            secondary_action_focused: Style::default()
                .fg(Color::White)
                .bg(colors.secondary)
                .bold(), // Make focused state visually distinct

            // Interactive element styles
            interactive: Style::default().fg(colors.text).bg(colors.surface),
            interactive_focused: Style::default().fg(colors.text).bg(colors.surface).bold(), // Make focused state visually distinct
            interactive_disabled: Style::default().fg(colors.text_muted).bg(colors.surface),

            // Text styles
            text: Style::default().fg(colors.text),
            text_muted: Style::default().fg(colors.text_muted),
            text_heading: Style::default().fg(colors.text).bold(),

            // Container styles
            surface: Style::default().bg(colors.background).fg(colors.text),
            surface_elevated: Style::default().bg(colors.surface).fg(colors.text),

            // State styles
            selected: Style::default().fg(Color::White).bg(colors.primary),
            hover: Style::default().fg(Color::White).bg(colors.primary).bold(),
            disabled: Style::default().fg(colors.text_muted),
        }
    }
}

/// A complete theme definition
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme name
    pub name: String,
    /// Semantic color palette
    pub colors: ThemeColors,
    /// Semantic style roles
    pub styles: ThemeStyles,
}

impl Theme {
    /// Create a new theme with the given name, colors, and styles
    pub fn new(name: impl Into<String>, colors: ThemeColors, styles: ThemeStyles) -> Self {
        Self {
            name: name.into(),
            colors,
            styles,
        }
    }

    /// Create the built-in dark theme
    pub fn dark() -> Self {
        let colors = ThemeColors::dark();
        let styles = ThemeStyles::dark(&colors);
        Self::new("dark", colors, styles)
    }

    /// Create the built-in light theme
    pub fn light() -> Self {
        let colors = ThemeColors::light();
        let styles = ThemeStyles::light(&colors);
        Self::new("light", colors, styles)
    }

    /// Create a theme builder for custom themes
    pub fn builder() -> ThemeBuilder {
        ThemeBuilder::new()
    }
}

/// Builder for creating custom themes
#[derive(Debug)]
pub struct ThemeBuilder {
    name: String,
    colors: Option<ThemeColors>,
    styles: Option<ThemeStyles>,
}

impl ThemeBuilder {
    /// Create a new theme builder
    pub fn new() -> Self {
        Self {
            name: "custom".to_string(),
            colors: None,
            styles: None,
        }
    }

    /// Set the theme name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the color palette
    pub fn colors(mut self, colors: ThemeColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set the style roles
    pub fn styles(mut self, styles: ThemeStyles) -> Self {
        self.styles = Some(styles);
        self
    }

    /// Build the theme, using dark theme defaults for unset fields
    pub fn build(self) -> Theme {
        let colors = self.colors.unwrap_or_else(ThemeColors::dark);
        let styles = self.styles.unwrap_or_else(|| ThemeStyles::dark(&colors));
        Theme::new(self.name, colors, styles)
    }
}

impl Default for ThemeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_creation() {
        let theme = Theme::dark();
        assert_eq!(theme.name, "dark");
    }

    #[test]
    fn test_light_theme_creation() {
        let theme = Theme::light();
        assert_eq!(theme.name, "light");
    }

    #[test]
    fn test_custom_theme_builder() {
        let theme = Theme::builder()
            .name("custom")
            .colors(ThemeColors::dark())
            .build();
        assert_eq!(theme.name, "custom");
    }
}
