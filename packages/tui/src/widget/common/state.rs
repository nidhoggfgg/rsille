//! Common widget state management
//!
//! This module provides shared state structures used by all interactive widgets,
//! reducing code duplication and providing a consistent interface.

use crate::style::Style;

/// Common state shared by all interactive widgets
///
/// This structure encapsulates the state fields that nearly every interactive
/// widget needs, eliminating the need to duplicate these fields in each widget.
///
/// # Examples
/// ```
/// use tui::widget::common::WidgetState;
///
/// let state = WidgetState::new();
/// assert!(!state.is_focused());
/// assert!(!state.is_disabled());
/// ```
#[derive(Debug, Clone, Default)]
pub struct WidgetState {
    /// Whether this widget currently has focus
    pub focused: bool,

    /// Whether this widget is disabled (cannot be interacted with)
    pub disabled: bool,

    /// Custom style that overrides theme defaults
    pub custom_style: Option<Style>,

    /// Custom style when focused that overrides theme focus defaults
    pub custom_focus_style: Option<Style>,
}

impl WidgetState {
    /// Create a new widget state with default values
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::WidgetState;
    ///
    /// let state = WidgetState::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new widget state with disabled status
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::WidgetState;
    ///
    /// let state = WidgetState::new().with_disabled(true);
    /// assert!(state.is_disabled());
    /// ```
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Create a new widget state with custom style
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::WidgetState;
    /// use tui::style::{Style, Color};
    ///
    /// let state = WidgetState::new()
    ///     .with_style(Style::default().fg(Color::Cyan));
    /// ```
    pub fn with_style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Create a new widget state with custom focus style
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::WidgetState;
    /// use tui::style::{Style, Color};
    ///
    /// let state = WidgetState::new()
    ///     .with_focus_style(Style::default().fg(Color::Cyan).bold());
    /// ```
    pub fn with_focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Check if the widget is currently focused
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::WidgetState;
    ///
    /// let mut state = WidgetState::new();
    /// assert!(!state.is_focused());
    ///
    /// state.set_focused(true);
    /// assert!(state.is_focused());
    /// ```
    #[inline]
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Check if the widget is currently disabled
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::WidgetState;
    ///
    /// let state = WidgetState::new().with_disabled(true);
    /// assert!(state.is_disabled());
    /// ```
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Set the focused state
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::WidgetState;
    ///
    /// let mut state = WidgetState::new();
    /// state.set_focused(true);
    /// assert!(state.is_focused());
    /// ```
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Set the disabled state
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::WidgetState;
    ///
    /// let mut state = WidgetState::new();
    /// state.set_disabled(true);
    /// assert!(state.is_disabled());
    /// ```
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Check if the widget can be focused
    ///
    /// A widget is focusable if it's not disabled.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::common::WidgetState;
    ///
    /// let state = WidgetState::new();
    /// assert!(state.is_focusable());
    ///
    /// let disabled_state = WidgetState::new().with_disabled(true);
    /// assert!(!disabled_state.is_focusable());
    /// ```
    #[inline]
    pub fn is_focusable(&self) -> bool {
        !self.disabled
    }
}
