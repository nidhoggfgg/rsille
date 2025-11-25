//! Style calculation and configuration for select widget

use crate::style::{Style, ThemeManager};
use ::render::style::Style as RenderStyle;

/// Style configuration for different select modes
#[derive(Debug, Clone, Copy)]
pub struct StyleConfig {
    /// Horizontal padding offset for content
    pub content_padding_x: u16,
    /// Vertical padding offset for content
    pub content_padding_y: u16,
    /// Border offset (0 for borderless, 1 for bordered)
    pub border_offset: u16,
}

impl StyleConfig {
    /// Create style config for borderless mode
    pub fn borderless() -> Self {
        Self {
            content_padding_x: 1,
            content_padding_y: 0,
            border_offset: 0,
        }
    }

    /// Create style config for bordered mode
    pub fn bordered() -> Self {
        Self {
            content_padding_x: 2,
            content_padding_y: 1,
            border_offset: 1,
        }
    }

    /// Get config based on borderless flag
    pub fn from_borderless(borderless: bool) -> Self {
        if borderless {
            Self::borderless()
        } else {
            Self::bordered()
        }
    }

    /// Calculate the trigger height
    pub fn trigger_height(&self) -> u16 {
        if self.border_offset == 0 {
            1 // Borderless: single line
        } else {
            3 // Bordered: border + content + border
        }
    }

    /// Calculate the dropdown border offset
    pub fn dropdown_border_offset(&self) -> u16 {
        if self.border_offset == 0 {
            0
        } else {
            2 // Top and bottom borders
        }
    }
}

/// Collection of render styles for dropdown items
#[derive(Debug, Clone, Copy)]
pub struct DropdownStyles {
    pub normal: RenderStyle,
    pub focused: RenderStyle,
    pub selected: RenderStyle,
    pub border: RenderStyle,
    pub background: RenderStyle,
    pub disabled: RenderStyle,
}

impl DropdownStyles {
    /// Get dropdown styles from theme
    pub fn from_theme() -> Self {
        ThemeManager::global().with_theme(|theme| Self {
            normal: Style::default()
                .fg(theme.colors.text)
                .bg(theme.colors.surface)
                .to_render_style(),
            focused: Style::default()
                .fg(theme.colors.text)
                .bg(theme.colors.focus_background)
                .to_render_style(),
            selected: Style::default()
                .fg(theme.colors.text)
                .bg(theme.colors.primary)
                .to_render_style(),
            border: Style::default()
                .fg(theme.colors.border)
                .bg(theme.colors.surface)
                .to_render_style(),
            background: Style::default().bg(theme.colors.surface).to_render_style(),
            disabled: theme.styles.disabled.to_render_style(),
        })
    }

    /// Get style for a specific item based on its state
    pub fn item_style(
        &self,
        is_disabled: bool,
        is_selected: bool,
        is_focused: bool,
    ) -> RenderStyle {
        if is_disabled {
            self.disabled
        } else if is_selected {
            self.selected
        } else if is_focused {
            self.focused
        } else {
            self.normal
        }
    }
}

/// Get trigger style based on focus state and custom styles
pub fn get_trigger_style(
    is_focused: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
) -> RenderStyle {
    if is_focused {
        ThemeManager::global().with_theme(|theme| {
            custom_focus_style
                .unwrap_or(theme.styles.interactive_focused)
                .to_render_style()
        })
    } else {
        ThemeManager::global().with_theme(|theme| {
            custom_style
                .unwrap_or(theme.styles.interactive)
                .to_render_style()
        })
    }
}

/// Get scrollbar styles from theme
pub fn get_scrollbar_styles() -> (RenderStyle, RenderStyle) {
    ThemeManager::global().with_theme(|theme| {
        (
            Style::default().fg(theme.colors.border).to_render_style(),
            Style::default().fg(theme.colors.info).to_render_style(),
        )
    })
}
