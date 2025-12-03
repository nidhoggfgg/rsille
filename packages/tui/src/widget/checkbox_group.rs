//! CheckboxGroup widget - interactive checkbox group component

use super::*;
use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::style::{Style, ThemeManager};
use crate::widget::common::{SelectableNavigation, StyleManager, WidgetState};
use std::sync::Arc;

/// Checkbox group layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckboxDirection {
    /// Vertical layout (options stacked vertically)
    #[default]
    Vertical,
    /// Horizontal layout (options arranged horizontally)
    Horizontal,
}

/// Interactive checkbox group widget
///
/// A checkbox group allows users to select multiple options from a list.
/// Supports both keyboard and mouse interaction, and can be laid out vertically or horizontally.
/// Within the group, use arrow keys to navigate. Between groups, use Tab/Shift+Tab.
///
/// # Examples
/// ```
/// use tui::widget::{CheckboxGroup, CheckboxDirection};
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     FeaturesChanged(usize, bool, Vec<bool>),
/// }
///
/// // Vertical layout (default)
/// let features = vec!["Dark Mode", "Notifications", "Auto Save"];
/// let checkbox_group = CheckboxGroup::new(features)
///     .checked(vec![true, false, true])
///     .on_change(|index, checked, states| Message::FeaturesChanged(index, checked, states));
///
/// // Horizontal layout with focused_index to preserve focus position
/// let options = vec!["A", "B", "C"];
/// let checkbox_horizontal = CheckboxGroup::new(options)
///     .direction(CheckboxDirection::Horizontal)
///     .focused_index(1)  // Focus on "B"
///     .on_change(|index, checked, states| Message::FeaturesChanged(index, checked, states));
/// ```
#[derive(Clone)]
pub struct CheckboxGroup<M = ()> {
    options: Vec<String>,
    checked: Vec<bool>,
    direction: CheckboxDirection,
    state: WidgetState,
    navigation: SelectableNavigation,
    on_change: Option<Arc<dyn Fn(usize, bool, Vec<bool>) -> M + Send + Sync>>,
}

impl<M> std::fmt::Debug for CheckboxGroup<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckboxGroup")
            .field("options", &self.options)
            .field("checked", &self.checked)
            .field("direction", &self.direction)
            .field("state", &self.state)
            .field("navigation", &self.navigation)
            .field("on_change", &self.on_change.is_some())
            .finish()
    }
}

impl<M> CheckboxGroup<M> {
    /// Create a new checkbox group with the specified options
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CheckboxGroup;
    ///
    /// let options = vec!["Option A", "Option B", "Option C"];
    /// let group = CheckboxGroup::<()>::new(options);
    /// ```
    pub fn new<S: Into<String>>(options: impl IntoIterator<Item = S>) -> Self {
        let options: Vec<String> = options.into_iter().map(|s| s.into()).collect();
        let checked = vec![false; options.len()];
        let navigation = SelectableNavigation::new(options.len(), options.len());
        Self {
            options,
            checked,
            direction: CheckboxDirection::default(),
            state: WidgetState::new(),
            navigation,
            on_change: None,
        }
    }

    /// Set the checked states for all options
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CheckboxGroup;
    ///
    /// let group = CheckboxGroup::<()>::new(vec!["A", "B", "C"])
    ///     .checked(vec![true, false, true]);
    /// ```
    pub fn checked(mut self, checked: Vec<bool>) -> Self {
        if checked.len() == self.options.len() {
            self.checked = checked;
        }
        self
    }

    /// Set the focused option index
    ///
    /// This is useful to preserve the focus position across re-renders.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CheckboxGroup;
    ///
    /// let group = CheckboxGroup::<()>::new(vec!["A", "B", "C"])
    ///     .focused_index(1); // Focus on "B"
    /// ```
    pub fn focused_index(mut self, index: usize) -> Self {
        if index < self.options.len() {
            self.navigation.set_focused_index(Some(index));
        }
        self
    }

    /// Set a custom change handler
    ///
    /// The handler will be called when any checkbox state changes.
    /// The handler receives: (index of toggled option, new checked state, all states).
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CheckboxGroup;
    ///
    /// #[derive(Clone)]
    /// enum Message { OptionToggled(usize, bool, Vec<bool>) }
    ///
    /// let group = CheckboxGroup::new(vec!["A", "B", "C"])
    ///     .on_change(|index, checked, states| Message::OptionToggled(index, checked, states));
    /// ```
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(usize, bool, Vec<bool>) -> M + Send + Sync + 'static,
    {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Set the disabled state
    ///
    /// Disabled checkbox groups cannot be interacted with and use muted styling.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CheckboxGroup;
    ///
    /// let group = CheckboxGroup::<()>::new(vec!["A", "B"])
    ///     .disabled(true);
    /// ```
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.set_disabled(disabled);
        self
    }

    /// Set the layout direction
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{CheckboxGroup, CheckboxDirection};
    ///
    /// let group = CheckboxGroup::<()>::new(vec!["A", "B", "C"])
    ///     .direction(CheckboxDirection::Horizontal);
    /// ```
    pub fn direction(mut self, direction: CheckboxDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set a custom style (overrides theme styling)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CheckboxGroup;
    /// use tui::style::{Style, Color};
    ///
    /// let group = CheckboxGroup::<()>::new(vec!["A", "B"])
    ///     .style(Style::default().fg(Color::Cyan));
    /// ```
    pub fn style(mut self, style: Style) -> Self {
        self.state = self.state.with_style(style);
        self
    }

    /// Set a custom focus style (overrides theme focus styling)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CheckboxGroup;
    /// use tui::style::{Style, Color};
    ///
    /// let group = CheckboxGroup::<()>::new(vec!["A", "B"])
    ///     .focus_style(Style::default().fg(Color::Cyan).bold());
    /// ```
    pub fn focus_style(mut self, style: Style) -> Self {
        self.state = self.state.with_focus_style(style);
        self
    }

    /// Get the effective style for a specific option
    fn get_option_style(&self, option_index: usize) -> Style {
        let focused_idx = self.navigation.focused_index();
        let is_focused_option = self.state.is_focused() && focused_idx == Some(option_index);

        // Temporarily create a state for this specific option
        let mut option_state = self.state.clone();
        option_state.set_focused(is_focused_option);

        StyleManager::interactive_style(&option_state)
    }

    /// Toggle an option by index and emit change event
    fn toggle_option(&mut self, index: usize) -> Vec<M> {
        if index < self.checked.len() {
            self.checked[index] = !self.checked[index];
            let new_checked = self.checked[index];
            if let Some(ref handler) = self.on_change {
                let message = handler(index, new_checked, self.checked.clone());
                vec![message]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    /// Render options vertically (one per line)
    fn render_vertical(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();

        for (index, option) in self.options.iter().enumerate() {
            if index as u16 >= area.height() {
                break; // No more space to render
            }

            let style = self.get_option_style(index);
            let render_style = style.to_render_style();

            // Render checkbox symbol
            let is_checked = self.checked.get(index).copied().unwrap_or(false);
            let checkbox_symbol = if is_checked { "[✓]" } else { "[ ]" };

            // Use success color (green) for checked checkbox to make it stand out
            let checkbox_style = if is_checked && !self.state.is_disabled() {
                ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.success)
                        .bold()
                        .to_render_style()
                })
            } else {
                render_style
            };

            // Render checkbox symbol with highlight when checked
            let _ = chunk.set_string(0, index as u16, checkbox_symbol, checkbox_style);

            // Render option label with a space after the checkbox
            // When this option is focused, make the label text more prominent with color
            if !option.is_empty() {
                let label_x = 4; // Position after "[X] "
                let focused_idx = self.navigation.focused_index();
                let is_focused_option = self.state.is_focused() && focused_idx == Some(index);
                let label_style = if is_focused_option && !self.state.is_disabled() {
                    // Use info color (blue/cyan) and bold when focused to make it stand out
                    ThemeManager::global().with_theme(|theme| {
                        Style::default()
                            .fg(theme.colors.info)
                            .bold()
                            .to_render_style()
                    })
                } else {
                    render_style
                };
                let _ = chunk.set_string(label_x, index as u16, option, label_style);
            }
        }
    }

    /// Render options horizontally (side by side)
    fn render_horizontal(&self, chunk: &mut render::chunk::Chunk) {
        use unicode_width::UnicodeWidthStr;
        let area = chunk.area();
        let mut current_x = 0u16;

        for (index, option) in self.options.iter().enumerate() {
            let style = self.get_option_style(index);
            let render_style = style.to_render_style();

            // Render checkbox symbol
            let is_checked = self.checked.get(index).copied().unwrap_or(false);
            let checkbox_symbol = if is_checked { "[✓]" } else { "[ ]" };

            // Calculate the width needed for this option
            let checkbox_width = 3; // "[X]" is 3 chars wide
            let space_width = 1;
            let label_width = option.width() as u16;
            let gap_width = 2; // Space between options
            let option_total_width = checkbox_width + space_width + label_width;

            // Check if we have enough space
            if current_x + option_total_width > area.width() {
                break; // No more space to render
            }

            // Use success color (green) for checked checkbox to make it stand out
            let checkbox_style = if is_checked && !self.state.is_disabled() {
                ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.success)
                        .bold()
                        .to_render_style()
                })
            } else {
                render_style
            };

            // Render checkbox symbol with highlight when checked
            let _ = chunk.set_string(current_x, 0, checkbox_symbol, checkbox_style);

            // Render option label with a space after the checkbox
            // When this option is focused, make the label text more prominent with color
            if !option.is_empty() {
                let label_x = current_x + 4; // Position after "[X] "
                let focused_idx = self.navigation.focused_index();
                let is_focused_option = self.state.is_focused() && focused_idx == Some(index);
                let label_style = if is_focused_option && !self.state.is_disabled() {
                    // Use info color (blue/cyan) and bold when focused to make it stand out
                    ThemeManager::global().with_theme(|theme| {
                        Style::default()
                            .fg(theme.colors.info)
                            .bold()
                            .to_render_style()
                    })
                } else {
                    render_style
                };
                let _ = chunk.set_string(label_x, 0, option, label_style);
            }

            // Move to next option position
            current_x += option_total_width + gap_width;
        }
    }
}

impl<M: Send + Sync> Widget<M> for CheckboxGroup<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        match self.direction {
            CheckboxDirection::Vertical => self.render_vertical(chunk),
            CheckboxDirection::Horizontal => self.render_horizontal(chunk),
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // Disabled checkbox groups don't handle events
        if self.state.is_disabled() {
            return EventResult::Ignored;
        }

        match event {
            // Keyboard events
            Event::Key(key_event) => {
                match key_event.code {
                    // Up/Left arrow moves focus to previous option
                    KeyCode::Up => {
                        if self.direction == CheckboxDirection::Vertical {
                            self.navigation
                                .focus_previous(|_| false, self.options.len());
                            return EventResult::consumed();
                        }
                    }
                    KeyCode::Left => {
                        if self.direction == CheckboxDirection::Horizontal {
                            self.navigation
                                .focus_previous(|_| false, self.options.len());
                            return EventResult::consumed();
                        }
                    }
                    // Down/Right arrow moves focus to next option
                    KeyCode::Down => {
                        if self.direction == CheckboxDirection::Vertical {
                            self.navigation.focus_next(|_| false, self.options.len());
                            return EventResult::consumed();
                        }
                    }
                    KeyCode::Right => {
                        if self.direction == CheckboxDirection::Horizontal {
                            self.navigation.focus_next(|_| false, self.options.len());
                            return EventResult::consumed();
                        }
                    }
                    // Enter or Space toggles the focused option
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(focused_idx) = self.navigation.focused_index() {
                            let messages = self.toggle_option(focused_idx);
                            return EventResult::Consumed(messages);
                        }
                    }
                    _ => {}
                }
            }

            // Mouse events
            Event::Mouse(mouse_event) => {
                if let MouseEventKind::Down(MouseButton::Left) = mouse_event.kind {
                    match self.direction {
                        CheckboxDirection::Vertical => {
                            // Click based on row position
                            let clicked_row = mouse_event.row;
                            if (clicked_row as usize) < self.options.len() {
                                let messages = self.toggle_option(clicked_row as usize);
                                return EventResult::Consumed(messages);
                            }
                        }
                        CheckboxDirection::Horizontal => {
                            // Click based on column position
                            use unicode_width::UnicodeWidthStr;
                            let mut current_x = 0u16;
                            let clicked_col = mouse_event.column;

                            for (index, option) in self.options.iter().enumerate() {
                                let checkbox_width = 3;
                                let space_width = 1;
                                let label_width = option.width() as u16;
                                let gap_width = 2;
                                let option_total_width = checkbox_width + space_width + label_width;

                                if clicked_col >= current_x
                                    && clicked_col < current_x + option_total_width
                                {
                                    let messages = self.toggle_option(index);
                                    return EventResult::Consumed(messages);
                                }

                                current_x += option_total_width + gap_width;
                            }
                        }
                    }
                }
            }

            _ => {}
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        use unicode_width::UnicodeWidthStr;

        match self.direction {
            CheckboxDirection::Vertical => {
                // Find the widest option
                let max_label_width = self
                    .options
                    .iter()
                    .map(|opt| opt.width() as u16)
                    .max()
                    .unwrap_or(0);

                // Checkbox symbol "[X]" is 3 characters wide, plus 1 space, plus label width
                let checkbox_width = 3;
                let space_width = 1;
                let total_width = checkbox_width + space_width + max_label_width;

                // Height is the number of options
                let height = self.options.len() as u16;

                Constraints {
                    min_width: total_width,
                    max_width: Some(total_width),
                    min_height: height,
                    max_height: Some(height),
                    flex: None,
                }
            }
            CheckboxDirection::Horizontal => {
                // Calculate total width needed for all options side by side
                let checkbox_width = 3;
                let space_width = 1;
                let gap_width = 2;

                let total_width: u16 = self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, opt)| {
                        let label_width = opt.width() as u16;
                        let option_width = checkbox_width + space_width + label_width;
                        if i < self.options.len() - 1 {
                            option_width + gap_width
                        } else {
                            option_width
                        }
                    })
                    .sum();

                // Height is always 1 for horizontal layout
                let height = 1;

                Constraints {
                    min_width: total_width,
                    max_width: Some(total_width),
                    min_height: height,
                    max_height: Some(height),
                    flex: None,
                }
            }
        }
    }

    fn focusable(&self) -> bool {
        // CheckboxGroup itself is not focusable - individual options are
        // Each option will be added to focus chain via build_focus_chain_recursive
        false
    }

    fn is_focused(&self) -> bool {
        self.state.is_focused()
    }

    fn set_focused(&mut self, focused: bool) {
        self.state.set_focused(focused);

        // When gaining focus, ensure focused_option is valid
        if focused {
            if let Some(idx) = self.navigation.focused_index() {
                if idx >= self.options.len() {
                    self.navigation.set_focused_index(Some(0));
                }
            } else {
                self.navigation.set_focused_index(Some(0));
            }
        }
    }

    fn build_focus_chain_recursive(
        &self,
        current_path: &mut Vec<usize>,
        chain: &mut Vec<crate::widget_id::WidgetId>,
    ) {
        // Skip if disabled or empty
        if self.state.is_disabled() || self.options.is_empty() {
            return;
        }

        // Add each option as a separate focusable item
        // Using virtual child indices to represent each option
        for option_idx in 0..self.options.len() {
            current_path.push(option_idx);
            let widget_id = crate::widget_id::WidgetId::from_path(current_path);
            chain.push(widget_id);
            current_path.pop();
        }
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_id: Option<crate::widget_id::WidgetId>,
    ) {
        // Check if focus is within this CheckboxGroup
        if let Some(focus) = focus_id {
            // Check each option to see if it matches the focused ID
            for option_idx in 0..self.options.len() {
                let mut option_path = current_path.to_vec();
                option_path.push(option_idx);
                let option_id = crate::widget_id::WidgetId::from_path(&option_path);

                if focus == option_id {
                    // This option is focused
                    self.state.set_focused(true);
                    self.navigation.set_focused_index(Some(option_idx));
                    return;
                }
            }
        }

        // Not focused
        self.state.set_focused(false);
    }
}

/// Create a new checkbox group widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message { StatesChanged(Vec<bool>) }
///
/// let group = checkbox_group(vec!["Dark Mode", "Notifications", "Auto Save"])
///     .checked(vec![true, false, true])
///     .on_change(|states| Message::StatesChanged(states));
/// ```
pub fn checkbox_group<M, S: Into<String>>(
    options: impl IntoIterator<Item = S>,
) -> CheckboxGroup<M> {
    CheckboxGroup::new(options)
}
