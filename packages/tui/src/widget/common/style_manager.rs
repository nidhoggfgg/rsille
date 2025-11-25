//! Style management utilities for widgets
//!
//! This module provides helper functions to compute effective styles
//! based on widget state and theme settings, reducing code duplication
//! across different widget implementations.

use super::state::WidgetState;
use crate::style::{Style, ThemeManager};

/// Style computation helper for widgets
///
/// This struct provides utility methods to compute the effective style
/// for a widget based on its state, custom styles, and theme settings.
pub struct StyleManager;

impl StyleManager {
    /// Compute the effective style for a widget
    ///
    /// This method handles the common pattern of:
    /// 1. Getting base style from theme based on state
    /// 2. Applying custom focus style if focused
    /// 3. Merging custom style
    ///
    /// # Arguments
    /// * `state` - The widget's current state
    /// * `theme_style_fn` - Function to get theme style based on state
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::{WidgetState, StyleManager};
    ///
    /// let state = WidgetState::new();
    /// let style = StyleManager::compute_style(&state, |theme, state| {
    ///     if state.is_disabled() {
    ///         theme.styles.disabled
    ///     } else if state.is_focused() {
    ///         theme.styles.interactive_focused
    ///     } else {
    ///         theme.styles.interactive
    ///     }
    /// });
    /// ```
    pub fn compute_style<F>(state: &WidgetState, theme_style_fn: F) -> Style
    where
        F: FnOnce(&crate::style::Theme, &WidgetState) -> Style,
    {
        // Get base theme style
        let base_style = ThemeManager::global().with_theme(|theme| theme_style_fn(theme, state));

        // If custom focus style is provided and widget is focused, apply it
        let style = if state.is_focused() {
            if let Some(ref focus_style) = state.custom_focus_style {
                focus_style.merge(base_style)
            } else {
                base_style
            }
        } else {
            base_style
        };

        // Merge custom style if provided (custom style takes precedence)
        state
            .custom_style
            .as_ref()
            .map(|s| s.merge(style))
            .unwrap_or(style)
    }

    /// Get style for interactive widgets with standard states
    ///
    /// This is a convenience method that handles the most common case:
    /// disabled -> focused -> normal styles.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::{WidgetState, StyleManager};
    ///
    /// let state = WidgetState::new();
    /// let style = StyleManager::interactive_style(&state);
    /// ```
    pub fn interactive_style(state: &WidgetState) -> Style {
        Self::compute_style(state, |theme, state| {
            if state.is_disabled() {
                theme.styles.disabled
            } else if state.is_focused() {
                theme.styles.interactive_focused
            } else {
                theme.styles.interactive
            }
        })
    }

    /// Get style for button-like widgets with hover states
    ///
    /// This method supports an additional hover state for widgets like buttons.
    ///
    /// # Arguments
    /// * `state` - The widget's current state
    /// * `is_hover` - Whether the widget is in hover state
    /// * `variant_normal` - Normal style for this variant
    /// * `variant_hover` - Hover style for this variant
    /// * `variant_focused` - Focused style for this variant
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::{WidgetState, StyleManager};
    /// use tui::style::Style;
    ///
    /// let state = WidgetState::new();
    /// let style = StyleManager::button_style(
    ///     &state,
    ///     false,
    ///     |theme| theme.styles.primary_action,
    ///     |theme| theme.styles.primary_action_hover,
    ///     |theme| theme.styles.primary_action_focused,
    /// );
    /// ```
    pub fn button_style<FN, FH, FF>(
        state: &WidgetState,
        is_hover: bool,
        variant_normal: FN,
        variant_hover: FH,
        variant_focused: FF,
    ) -> Style
    where
        FN: FnOnce(&crate::style::Theme) -> Style,
        FH: FnOnce(&crate::style::Theme) -> Style,
        FF: FnOnce(&crate::style::Theme) -> Style,
    {
        Self::compute_style(state, |theme, state| {
            if state.is_disabled() {
                theme.styles.disabled
            } else if state.is_focused() {
                variant_focused(theme)
            } else if is_hover {
                variant_hover(theme)
            } else {
                variant_normal(theme)
            }
        })
    }

    /// Get border color based on focus state
    ///
    /// This is a common pattern for bordered widgets.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::{WidgetState, StyleManager};
    ///
    /// let state = WidgetState::new();
    /// let border_color = StyleManager::border_color(&state);
    /// ```
    pub fn border_color(state: &WidgetState) -> crate::style::Color {
        ThemeManager::global().with_theme(|theme| {
            if state.is_focused() {
                theme.colors.focus_ring
            } else {
                theme.colors.border
            }
        })
    }

    /// Get render style (converted from Style) for a widget
    ///
    /// This combines style computation and conversion to render::style::Style.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::{WidgetState, StyleManager};
    ///
    /// let state = WidgetState::new();
    /// let render_style = StyleManager::render_style(&state);
    /// ```
    pub fn render_style(state: &WidgetState) -> render::style::Style {
        Self::interactive_style(state).to_render_style()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_style_normal() {
        let state = WidgetState::new();
        // Should return theme's interactive style
        assert!(!state.is_focused());
        assert!(!state.is_disabled());
    }

    #[test]
    fn test_interactive_style_focused() {
        let mut state = WidgetState::new();
        state.set_focused(true);
        // Should return theme's interactive_focused style
        assert!(state.is_focused());
    }

    #[test]
    fn test_interactive_style_disabled() {
        let state = WidgetState::new().with_disabled(true);
        // Should return theme's disabled style
        assert!(state.is_disabled());
    }

    #[test]
    fn test_border_color_normal() {
        let state = WidgetState::new();
        // Should return theme's border color
        assert!(!state.is_focused());
    }

    #[test]
    fn test_border_color_focused() {
        let mut state = WidgetState::new();
        state.set_focused(true);
        // Should return theme's focus_ring color
        assert!(state.is_focused());
    }
}
