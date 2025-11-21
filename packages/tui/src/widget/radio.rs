//! RadioGroup widget - interactive radio button group component

use super::*;
use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::style::{Style, ThemeManager};
use std::sync::Arc;

/// Interactive radio button group widget
///
/// A radio group allows users to select one option from a list of mutually exclusive choices.
/// Supports both keyboard and mouse interaction.
///
/// # Examples
/// ```
/// use tui::widget::RadioGroup;
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     SizeSelected(usize),
/// }
///
/// let options = vec!["Small", "Medium", "Large"];
/// let radio = RadioGroup::new(options)
///     .selected(1) // Select "Medium" by default
///     .on_change(|index| Message::SizeSelected(index));
/// ```
#[derive(Clone)]
pub struct RadioGroup<M = ()> {
    options: Vec<String>,
    selected_index: Option<usize>,
    focused_option: usize,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    disabled: bool,
    focused: bool,
    on_change: Option<Arc<dyn Fn(usize) -> M + Send + Sync>>,
}

impl<M> std::fmt::Debug for RadioGroup<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioGroup")
            .field("options", &self.options)
            .field("selected_index", &self.selected_index)
            .field("focused_option", &self.focused_option)
            .field("custom_style", &self.custom_style)
            .field("custom_focus_style", &self.custom_focus_style)
            .field("disabled", &self.disabled)
            .field("focused", &self.focused)
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
            custom_style: None,
            custom_focus_style: None,
            disabled: false,
            focused: false,
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

    /// Set the disabled state
    ///
    /// Disabled radio groups cannot be interacted with and use muted styling.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::RadioGroup;
    ///
    /// let radio = RadioGroup::<()>::new(vec!["A", "B"])
    ///     .disabled(true);
    /// ```
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set a custom style (overrides theme styling)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::RadioGroup;
    /// use tui::style::{Style, Color};
    ///
    /// let radio = RadioGroup::<()>::new(vec!["A", "B"])
    ///     .style(Style::default().fg(Color::Cyan));
    /// ```
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Set a custom focus style (overrides theme focus styling)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::RadioGroup;
    /// use tui::style::{Style, Color};
    ///
    /// let radio = RadioGroup::<()>::new(vec!["A", "B"])
    ///     .focus_style(Style::default().fg(Color::Cyan).bold());
    /// ```
    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Get the effective style for a specific option
    fn get_option_style(&self, option_index: usize) -> Style {
        let is_focused_option = self.focused && option_index == self.focused_option;

        // Get base theme style based on state
        let base_style = ThemeManager::global().with_theme(|theme| {
            // Disabled state takes priority
            if self.disabled {
                return theme.styles.disabled;
            }

            // Focused option gets focus styling
            if is_focused_option {
                return theme.styles.interactive_focused;
            }

            // Normal state
            theme.styles.interactive
        });

        // If custom focus style is provided and this option is focused, use it
        if is_focused_option {
            if let Some(ref focus_style) = self.custom_focus_style {
                return focus_style.merge(base_style);
            }
        }

        // Merge custom style if provided (custom style takes precedence)
        self.custom_style
            .as_ref()
            .map(|s| s.merge(base_style))
            .unwrap_or(base_style)
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
}

impl<M: Send + Sync> Widget<M> for RadioGroup<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        // Render each option on a separate line
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
            let radio_style = if is_selected && !self.disabled {
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
                let is_focused_option = self.focused && index == self.focused_option;
                let label_style = if is_focused_option && !self.disabled {
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

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // Disabled radio groups don't handle events
        if self.disabled {
            return EventResult::Ignored;
        }

        match event {
            // Keyboard events
            Event::Key(key_event) => {
                match key_event.code {
                    // Up arrow moves focus to previous option
                    KeyCode::Up => {
                        self.focus_previous();
                        return EventResult::consumed();
                    }
                    // Down arrow moves focus to next option
                    KeyCode::Down => {
                        self.focus_next();
                        return EventResult::consumed();
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
                match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Determine which option was clicked based on mouse y position
                        // Note: This assumes the radio group is rendered at the top-left of its area
                        // In a real implementation with area tracking, we'd need proper bounds checking
                        let clicked_row = mouse_event.row;
                        if (clicked_row as usize) < self.options.len() {
                            let messages = self.select_option(clicked_row as usize);
                            return EventResult::Consumed(messages);
                        }
                    }
                    _ => {}
                }
            }

            _ => {}
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        use unicode_width::UnicodeWidthStr;

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

    fn focusable(&self) -> bool {
        // Radio groups are focusable unless disabled or empty
        !self.disabled && !self.options.is_empty()
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;

        // When gaining focus, set focused_option to the selected item if there is one
        if focused {
            if let Some(selected) = self.selected_index {
                self.focused_option = selected;
            }
        }
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
