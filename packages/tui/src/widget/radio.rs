//! RadioGroup widget - interactive radio button group component

use super::*;
use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::style::{Style, ThemeManager};
use crate::widget::common::{StatefulWidgetBuilder, StyleManager, WidgetState};
use std::sync::Arc;

/// Radio group layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RadioDirection {
    /// Vertical layout (options stacked vertically)
    #[default]
    Vertical,
    /// Horizontal layout (options arranged horizontally)
    Horizontal,
}

/// Interactive radio button group widget
///
/// A radio group allows users to select one option from a list of mutually exclusive choices.
/// Supports both keyboard and mouse interaction, and can be laid out vertically or horizontally.
///
/// # Examples
/// ```
/// use tui::widget::{RadioGroup, RadioDirection};
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     SizeSelected(usize),
/// }
///
/// // Vertical layout (default)
/// let options = vec!["Small", "Medium", "Large"];
/// let radio_vertical = RadioGroup::new(options)
///     .selected(1) // Select "Medium" by default
///     .on_change(|index| Message::SizeSelected(index));
///
/// // Horizontal layout
/// let options = vec!["S", "M", "L", "XL"];
/// let radio_horizontal = RadioGroup::new(options)
///     .direction(RadioDirection::Horizontal)
///     .selected(1)
///     .on_change(|index| Message::SizeSelected(index));
/// ```
#[derive(Clone)]
pub struct RadioGroup<M = ()> {
    options: Vec<String>,
    selected_index: Option<usize>,
    focused_option: usize,
    direction: RadioDirection,
    state: WidgetState,
    on_change: Option<Arc<dyn Fn(usize) -> M + Send + Sync>>,
}

impl<M> std::fmt::Debug for RadioGroup<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioGroup")
            .field("options", &self.options)
            .field("selected_index", &self.selected_index)
            .field("focused_option", &self.focused_option)
            .field("direction", &self.direction)
            .field("state", &self.state)
            .field("on_change", &self.on_change.is_some())
            .finish()
    }
}

impl<M> RadioGroup<M> {
    /// Create a new radio group with the specified options
    ///
    /// # Examples
    /// ```
    /// use tui::widget::RadioGroup;
    ///
    /// let options = vec!["Option A", "Option B", "Option C"];
    /// let radio = RadioGroup::<()>::new(options);
    /// ```
    pub fn new<S: Into<String>>(options: impl IntoIterator<Item = S>) -> Self {
        let options: Vec<String> = options.into_iter().map(|s| s.into()).collect();
        Self {
            options,
            selected_index: None,
            focused_option: 0,
            direction: RadioDirection::default(),
            state: WidgetState::new(),
            on_change: None,
        }
    }

    /// Set the selected option by index
    ///
    /// # Examples
    /// ```
    /// use tui::widget::RadioGroup;
    ///
    /// let radio = RadioGroup::<()>::new(vec!["A", "B", "C"])
    ///     .selected(1); // Select "B"
    /// ```
    pub fn selected(mut self, index: usize) -> Self {
        if index < self.options.len() {
            self.selected_index = Some(index);
            self.focused_option = index;
        }
        self
    }

    /// Set a custom change handler
    ///
    /// The handler will be called when a different option is selected.
    /// The handler receives the index of the newly selected option.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::RadioGroup;
    ///
    /// #[derive(Clone)]
    /// enum Message { OptionSelected(usize) }
    ///
    /// let radio = RadioGroup::new(vec!["A", "B", "C"])
    ///     .on_change(|index| Message::OptionSelected(index));
    /// ```
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(usize) -> M + Send + Sync + 'static,
    {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Set the layout direction
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{RadioGroup, RadioDirection};
    ///
    /// let radio = RadioGroup::<()>::new(vec!["S", "M", "L"])
    ///     .direction(RadioDirection::Horizontal);
    /// ```
    pub fn direction(mut self, direction: RadioDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Get the effective style for a specific option
    fn get_option_style(&self, option_index: usize) -> Style {
        let is_focused_option = self.state.is_focused() && option_index == self.focused_option;

        // Create temporary state for this option
        let mut option_state = self.state.clone();
        option_state.set_focused(is_focused_option);

        // Use StyleManager to compute style
        StyleManager::interactive_style(&option_state)
    }

    /// Select an option by index and emit change event
    fn select_option(&mut self, index: usize) -> Vec<M> {
        // Don't emit change event if the same option is selected
        if self.selected_index == Some(index) {
            return vec![];
        }

        self.selected_index = Some(index);
        if let Some(ref handler) = self.on_change {
            let message = handler(index);
            vec![message]
        } else {
            vec![]
        }
    }

    /// Move the keyboard focus to the previous option
    fn focus_previous(&mut self) {
        if self.focused_option > 0 {
            self.focused_option -= 1;
        }
    }

    /// Move the keyboard focus to the next option
    fn focus_next(&mut self) {
        if self.focused_option < self.options.len().saturating_sub(1) {
            self.focused_option += 1;
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

            // Render radio symbol
            let is_selected = self.selected_index == Some(index);
            let radio_symbol = if is_selected { "(●)" } else { "( )" };

            // Use success color (green) for selected radio button to make it stand out
            let radio_style = if is_selected && !self.state.is_disabled() {
                ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.success)
                        .bold()
                        .to_render_style()
                })
            } else {
                render_style
            };

            // Render radio symbol with highlight when selected
            let _ = chunk.set_string(0, index as u16, radio_symbol, radio_style);

            // Render option label with a space after the radio button
            // When this option is focused, make the label text more prominent with color
            if !option.is_empty() {
                let label_x = 4; // Position after "(X) "
                let is_focused_option = self.state.is_focused() && index == self.focused_option;
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

            // Render radio symbol
            let is_selected = self.selected_index == Some(index);
            let radio_symbol = if is_selected { "(●)" } else { "( )" };

            // Calculate the width needed for this option
            let radio_width = 3; // "(X)" is 3 chars wide
            let space_width = 1;
            let label_width = option.width() as u16;
            let gap_width = 2; // Space between options
            let option_total_width = radio_width + space_width + label_width;

            // Check if we have enough space
            if current_x + option_total_width > area.width() {
                break; // No more space to render
            }

            // Use success color (green) for selected radio button to make it stand out
            let radio_style = if is_selected && !self.state.is_disabled() {
                ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.success)
                        .bold()
                        .to_render_style()
                })
            } else {
                render_style
            };

            // Render radio symbol with highlight when selected
            let _ = chunk.set_string(current_x, 0, radio_symbol, radio_style);

            // Render option label with a space after the radio button
            // When this option is focused, make the label text more prominent with color
            if !option.is_empty() {
                let label_x = current_x + 4; // Position after "(X) "
                let is_focused_option = self.state.is_focused() && index == self.focused_option;
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

impl<M: Send + Sync> Widget<M> for RadioGroup<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        match self.direction {
            RadioDirection::Vertical => self.render_vertical(chunk),
            RadioDirection::Horizontal => self.render_horizontal(chunk),
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // Disabled radio groups don't handle events
        if self.state.is_disabled() {
            return EventResult::Ignored;
        }

        match event {
            // Keyboard events
            Event::Key(key_event) => {
                match key_event.code {
                    // Up/Left arrow moves focus to previous option
                    KeyCode::Up => {
                        if self.direction == RadioDirection::Vertical {
                            self.focus_previous();
                            return EventResult::consumed();
                        }
                    }
                    KeyCode::Left => {
                        if self.direction == RadioDirection::Horizontal {
                            self.focus_previous();
                            return EventResult::consumed();
                        }
                    }
                    // Down/Right arrow moves focus to next option
                    KeyCode::Down => {
                        if self.direction == RadioDirection::Vertical {
                            self.focus_next();
                            return EventResult::consumed();
                        }
                    }
                    KeyCode::Right => {
                        if self.direction == RadioDirection::Horizontal {
                            self.focus_next();
                            return EventResult::consumed();
                        }
                    }
                    // Enter or Space selects the focused option
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        let messages = self.select_option(self.focused_option);
                        return EventResult::Consumed(messages);
                    }
                    _ => {}
                }
            }

            // Mouse events
            Event::Mouse(mouse_event) => {
                if let MouseEventKind::Down(MouseButton::Left) = mouse_event.kind {
                    match self.direction {
                        RadioDirection::Vertical => {
                            // Click based on row position
                            let clicked_row = mouse_event.row;
                            if (clicked_row as usize) < self.options.len() {
                                let messages = self.select_option(clicked_row as usize);
                                return EventResult::Consumed(messages);
                            }
                        }
                        RadioDirection::Horizontal => {
                            // Click based on column position (approximate)
                            // This is a simplified implementation
                            use unicode_width::UnicodeWidthStr;
                            let mut current_x = 0u16;
                            let clicked_col = mouse_event.column;

                            for (index, option) in self.options.iter().enumerate() {
                                let radio_width = 3;
                                let space_width = 1;
                                let label_width = option.width() as u16;
                                let gap_width = 2;
                                let option_total_width = radio_width + space_width + label_width;

                                if clicked_col >= current_x
                                    && clicked_col < current_x + option_total_width
                                {
                                    let messages = self.select_option(index);
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
            RadioDirection::Vertical => {
                // Find the widest option
                let max_label_width = self
                    .options
                    .iter()
                    .map(|opt| opt.width() as u16)
                    .max()
                    .unwrap_or(0);

                // Radio symbol "(X)" is 3 characters wide, plus 1 space, plus label width
                let radio_width = 3;
                let space_width = 1;
                let total_width = radio_width + space_width + max_label_width;

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
            RadioDirection::Horizontal => {
                // Calculate total width needed for all options side by side
                let radio_width = 3;
                let space_width = 1;
                let gap_width = 2;

                let total_width: u16 = self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, opt)| {
                        let label_width = opt.width() as u16;
                        let option_width = radio_width + space_width + label_width;
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
        // RadioGroup is focusable unless disabled
        self.state.is_focusable()
    }

    fn is_focused(&self) -> bool {
        self.state.is_focused()
    }

    fn set_focused(&mut self, focused: bool) {
        self.state.set_focused(focused);

        // When gaining focus, set focused_option to the selected item if there is one
        if focused {
            if let Some(selected) = self.selected_index {
                self.focused_option = selected;
            }
        }
    }

    fn build_focus_chain_recursive(
        &self,
        current_path: &mut Vec<usize>,
        chain: &mut Vec<crate::widget_id::WidgetId>,
    ) {
        use smallvec::SmallVec;

        // Skip if disabled or empty
        if self.state.is_disabled() || self.options.is_empty() {
            return;
        }

        // Add each option as a separate focusable item
        // Using virtual child indices to represent each option
        for option_idx in 0..self.options.len() {
            current_path.push(option_idx);
            chain.push(crate::widget_id::WidgetId::from_path(SmallVec::from_slice(current_path)));
            current_path.pop();
        }
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_id: Option<crate::widget_id::WidgetId>,
    ) {
        // Check if focus is within this RadioGroup
        if let Some(focus) = focus_id {
            let focus_path = focus.path();
            if focus_path.starts_with(current_path) && focus_path.len() == current_path.len() + 1 {
                // Focus is on one of our options
                let option_idx = focus_path[current_path.len()];
                if option_idx < self.options.len() {
                    self.state.set_focused(true);
                    self.focused_option = option_idx;
                    return;
                }
            }
        }

        // Not focused
        self.state.set_focused(false);
    }
}

// Implement StatefulWidgetBuilder to provide common builder methods
impl<M> StatefulWidgetBuilder for RadioGroup<M> {
    fn widget_state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }
}

/// Create a new radio group widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message { Selected(usize) }
///
/// let rg = radio_group(vec!["Small", "Medium", "Large"])
///     .selected(1)
///     .on_change(|index| Message::Selected(index));
/// ```
pub fn radio_group<M, S: Into<String>>(options: impl IntoIterator<Item = S>) -> RadioGroup<M> {
    RadioGroup::new(options)
}
