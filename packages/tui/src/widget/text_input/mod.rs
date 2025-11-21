//! TextInput widget - text input component

use super::*;
use crate::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::style::{BorderStyle, Style, ThemeManager};
use std::sync::Arc;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Text input style variants
///
/// Different visual styles for text inputs based on their purpose and context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputVariant {
    /// Default bordered input (single line border)
    Default,
    /// Borderless input (bottom border only)
    Borderless,
    /// Password input (masks characters as dots)
    Password,
}

impl Default for TextInputVariant {
    fn default() -> Self {
        TextInputVariant::Default
    }
}

/// Text input widget
///
/// A single-line text input field with support for editing, cursor navigation,
/// and keyboard interaction.
///
/// # Examples
/// ```
/// use tui::widget::{TextInput, TextInputVariant};
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     TextChanged(String),
///     Submit,
/// }
///
/// let input = TextInput::new()
///     .variant(TextInputVariant::Default)
///     .placeholder("Enter your name...")
///     .value("John")
///     .on_change(|text| Message::TextChanged(text))
///     .on_submit(|text| Message::Submit);
///
/// let password = TextInput::new()
///     .variant(TextInputVariant::Password)
///     .placeholder("Enter password...")
///     .on_change(|text| Message::TextChanged(text));
/// ```
#[derive(Clone)]
pub struct TextInput<M = ()> {
    // Content
    value: String,
    placeholder: Option<String>,

    // Cursor state (in byte index)
    cursor_position: usize,

    // Configuration
    variant: TextInputVariant,
    disabled: bool,

    // State
    focused: bool,

    // Styling
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,

    // Event handlers
    on_change: Option<Arc<dyn Fn(String) -> M + Send + Sync>>,
    on_submit: Option<Arc<dyn Fn(String) -> M + Send + Sync>>,
}

impl<M> std::fmt::Debug for TextInput<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInput")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("cursor_position", &self.cursor_position)
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .field("focused", &self.focused)
            .field("custom_style", &self.custom_style)
            .field("custom_focus_style", &self.custom_focus_style)
            .field("on_change", &self.on_change.is_some())
            .field("on_submit", &self.on_submit.is_some())
            .finish()
    }
}

impl<M> TextInput<M> {
    /// Create a new text input
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    ///
    /// let input = TextInput::<()>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: None,
            cursor_position: 0,
            variant: TextInputVariant::default(),
            disabled: false,
            focused: false,
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            on_submit: None,
        }
    }

    /// Set the input variant
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{TextInput, TextInputVariant};
    ///
    /// let input = TextInput::<()>::new()
    ///     .variant(TextInputVariant::Password);
    /// ```
    pub fn variant(mut self, variant: TextInputVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the input value
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    ///
    /// let input = TextInput::<()>::new()
    ///     .value("Hello");
    /// ```
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        // Move cursor to end of value
        self.cursor_position = self.value.len();
        self
    }

    /// Set the placeholder text
    ///
    /// Placeholder is shown when the input is empty.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    ///
    /// let input = TextInput::<()>::new()
    ///     .placeholder("Enter text...");
    /// ```
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the disabled state
    ///
    /// Disabled inputs cannot be interacted with.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    ///
    /// let input = TextInput::<()>::new()
    ///     .disabled(true);
    /// ```
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set a custom style
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    /// use tui::style::{Style, Color};
    ///
    /// let input = TextInput::<()>::new()
    ///     .style(Style::default().fg(Color::Cyan));
    /// ```
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Set a custom focus style
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    /// use tui::style::{Style, Color};
    ///
    /// let input = TextInput::<()>::new()
    ///     .focus_style(Style::default().fg(Color::Cyan).bold());
    /// ```
    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Set the change handler
    ///
    /// Called whenever the text changes.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    ///
    /// #[derive(Clone)]
    /// enum Message { TextChanged(String) }
    ///
    /// let input = TextInput::new()
    ///     .on_change(|text| Message::TextChanged(text));
    /// ```
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + Send + Sync + 'static,
    {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Set the submit handler
    ///
    /// Called when Enter is pressed.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TextInput;
    ///
    /// #[derive(Clone)]
    /// enum Message { Submit(String) }
    ///
    /// let input = TextInput::new()
    ///     .on_submit(|text| Message::Submit(text));
    /// ```
    pub fn on_submit<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + Send + Sync + 'static,
    {
        self.on_submit = Some(Arc::new(handler));
        self
    }

    /// Get the effective style based on current state and variant
    fn get_style(&self) -> Style {
        let base_style = ThemeManager::global().with_theme(|theme| {
            if self.disabled {
                theme.styles.interactive_disabled
            } else if self.focused {
                theme.styles.interactive_focused
            } else {
                theme.styles.interactive
            }
        });

        // If custom focus style is provided and input is focused, use it
        if self.focused {
            if let Some(ref focus_style) = self.custom_focus_style {
                return focus_style.merge(base_style);
            }
        }

        // Merge custom style if provided
        self.custom_style
            .as_ref()
            .map(|s| s.merge(base_style))
            .unwrap_or(base_style)
    }

    /// Insert a character at the cursor position
    fn insert_char(&mut self, c: char) -> Vec<M> {
        self.value.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
        self.emit_change()
    }

    /// Delete character before cursor (Backspace)
    fn delete_before_cursor(&mut self) -> Vec<M> {
        if self.cursor_position > 0 {
            // Find the previous character boundary
            let mut idx = self.cursor_position - 1;
            while idx > 0 && !self.value.is_char_boundary(idx) {
                idx -= 1;
            }
            self.value.remove(idx);
            self.cursor_position = idx;
            self.emit_change()
        } else {
            Vec::new()
        }
    }

    /// Delete character at cursor (Delete)
    fn delete_at_cursor(&mut self) -> Vec<M> {
        if self.cursor_position < self.value.len() {
            self.value.remove(self.cursor_position);
            self.emit_change()
        } else {
            Vec::new()
        }
    }

    /// Move cursor left
    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            let mut idx = self.cursor_position - 1;
            while idx > 0 && !self.value.is_char_boundary(idx) {
                idx -= 1;
            }
            self.cursor_position = idx;
        }
    }

    /// Move cursor right
    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.value.len() {
            let mut idx = self.cursor_position + 1;
            while idx < self.value.len() && !self.value.is_char_boundary(idx) {
                idx += 1;
            }
            self.cursor_position = idx;
        }
    }

    /// Move cursor to start
    fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to end
    fn move_cursor_end(&mut self) {
        self.cursor_position = self.value.len();
    }

    /// Emit change event
    fn emit_change(&self) -> Vec<M> {
        if let Some(ref handler) = self.on_change {
            vec![handler(self.value.clone())]
        } else {
            Vec::new()
        }
    }

    /// Emit submit event
    fn emit_submit(&self) -> Vec<M> {
        if let Some(ref handler) = self.on_submit {
            vec![handler(self.value.clone())]
        } else {
            Vec::new()
        }
    }

    /// Render full border (for Default, Filled, Search variants)
    fn render_full_border(
        &self,
        chunk: &mut render::chunk::Chunk,
        width: u16,
        height: u16,
        border_style: render::style::Style,
    ) {
        let border_chars = BorderStyle::Single.chars();

        // Top border
        let _ = chunk.set_char(0, 0, border_chars.top_left, border_style);
        let _ = chunk.set_char(width - 1, 0, border_chars.top_right, border_style);
        for x in 1..width - 1 {
            let _ = chunk.set_char(x, 0, border_chars.horizontal, border_style);
        }

        // Bottom border
        let _ = chunk.set_char(0, height - 1, border_chars.bottom_left, border_style);
        let _ = chunk.set_char(
            width - 1,
            height - 1,
            border_chars.bottom_right,
            border_style,
        );
        for x in 1..width - 1 {
            let _ = chunk.set_char(x, height - 1, border_chars.horizontal, border_style);
        }

        // Side borders for middle rows
        for y in 1..height - 1 {
            let _ = chunk.set_char(0, y, border_chars.vertical, border_style);
            let _ = chunk.set_char(width - 1, y, border_chars.vertical, border_style);
        }
    }

    /// Render bottom border only (for Borderless variant)
    fn render_bottom_border(
        &self,
        chunk: &mut render::chunk::Chunk,
        width: u16,
        height: u16,
        border_style: render::style::Style,
    ) {
        let border_chars = BorderStyle::Single.chars();

        // Only draw horizontal line at bottom
        for x in 0..width {
            let _ = chunk.set_char(x, height - 1, border_chars.horizontal, border_style);
        }
    }
}

impl<M> Default for TextInput<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Send + Sync> Widget<M> for TextInput<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        let width = area.width();
        let height = area.height();

        if width < 4 || height < 3 {
            return;
        }

        let style = self.get_style();
        let render_style = style.to_render_style();

        // Get border color
        let border_style = ThemeManager::global().with_theme(|theme| {
            if self.focused {
                Style::default()
                    .fg(theme.colors.focus_ring)
                    .to_render_style()
            } else {
                Style::default().fg(theme.colors.border).to_render_style()
            }
        });

        // Render based on variant
        match self.variant {
            TextInputVariant::Default | TextInputVariant::Password => {
                // Render full border
                self.render_full_border(chunk, width, height, border_style);
            }
            TextInputVariant::Borderless => {
                // Render only bottom border
                self.render_bottom_border(chunk, width, height, border_style);
            }
        }

        // Text content area - varies by variant
        let (text_y, text_start_x, available_width) = match self.variant {
            TextInputVariant::Borderless => {
                // Borderless: no top/side borders, just padding
                let text_y = 1;
                let text_start_x = 1u16;
                let available_width = (width - 2) as usize;
                (text_y, text_start_x, available_width)
            }
            _ => {
                // All other variants: inside the border with padding
                let text_y = 1;
                let text_start_x = 2u16;
                let available_width = (width - 4) as usize;
                (text_y, text_start_x, available_width)
            }
        };

        if available_width == 0 {
            return;
        }

        if self.value.is_empty() {
            // Show placeholder only when not focused
            if !self.focused {
                if let Some(ref placeholder) = self.placeholder {
                    let placeholder_style = ThemeManager::global().with_theme(|theme| {
                        Style::default()
                            .fg(theme.colors.text_muted)
                            .to_render_style()
                    });

                    let display_text: String = placeholder.chars().take(available_width).collect();
                    let _ =
                        chunk.set_string(text_start_x, text_y, &display_text, placeholder_style);
                }
            } else {
                // Show cursor at start position when focused
                let cursor_style = ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.background)
                        .bg(theme.colors.text)
                        .to_render_style()
                });
                let _ = chunk.set_char(text_start_x, text_y, ' ', cursor_style);
            }
        } else {
            // Calculate cursor visual position
            let text_before_cursor = &self.value[..self.cursor_position];
            let cursor_visual_pos = text_before_cursor.width();

            // For password variant, mask the text
            let display_value = if self.variant == TextInputVariant::Password {
                "•".repeat(self.value.chars().count())
            } else {
                self.value.clone()
            };

            // Display text (truncate if needed)
            let display_text: String = if display_value.width() > available_width {
                let mut result = String::new();
                let mut w = 0;
                for ch in display_value.chars() {
                    let ch_w = ch.width().unwrap_or(0);
                    if w + ch_w > available_width {
                        break;
                    }
                    result.push(ch);
                    w += ch_w;
                }
                result
            } else {
                display_value
            };

            let _ = chunk.set_string(text_start_x, text_y, &display_text, render_style);

            // Render cursor when focused
            if self.focused && cursor_visual_pos <= available_width {
                let cursor_x = text_start_x + cursor_visual_pos as u16;

                // Use block cursor style (inverse video)
                let cursor_style = ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.background)
                        .bg(theme.colors.text)
                        .to_render_style()
                });

                let cursor_char = if self.cursor_position >= self.value.len() {
                    ' ' // Block cursor at end
                } else if self.variant == TextInputVariant::Password {
                    '•' // Show bullet for password
                } else {
                    self.value[self.cursor_position..]
                        .chars()
                        .next()
                        .unwrap_or(' ')
                };

                let _ = chunk.set_char(cursor_x, text_y, cursor_char, cursor_style);
            }
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // Disabled inputs don't handle events
        if self.disabled {
            return EventResult::Ignored;
        }

        match event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                match code {
                    // Character input
                    KeyCode::Char(c) => {
                        // Check for Ctrl combinations
                        if modifiers.contains(KeyModifiers::CONTROL) {
                            match c {
                                'a' => {
                                    // Select all (not implemented in MVP, just move to end)
                                    self.move_cursor_end();
                                    return EventResult::Consumed(Vec::new());
                                }
                                _ => return EventResult::Ignored,
                            }
                        } else {
                            // Regular character input
                            let messages = self.insert_char(*c);
                            return EventResult::Consumed(messages);
                        }
                    }

                    // Backspace
                    KeyCode::Backspace => {
                        let messages = self.delete_before_cursor();
                        return EventResult::Consumed(messages);
                    }

                    // Delete
                    KeyCode::Delete => {
                        let messages = self.delete_at_cursor();
                        return EventResult::Consumed(messages);
                    }

                    // Arrow keys
                    KeyCode::Left => {
                        self.move_cursor_left();
                        return EventResult::Consumed(Vec::new());
                    }
                    KeyCode::Right => {
                        self.move_cursor_right();
                        return EventResult::Consumed(Vec::new());
                    }

                    // Home/End
                    KeyCode::Home => {
                        self.move_cursor_home();
                        return EventResult::Consumed(Vec::new());
                    }
                    KeyCode::End => {
                        self.move_cursor_end();
                        return EventResult::Consumed(Vec::new());
                    }

                    // Enter - submit
                    KeyCode::Enter => {
                        let messages = self.emit_submit();
                        return EventResult::Consumed(messages);
                    }

                    _ => {}
                }
            }
            _ => {}
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        // Fixed size: 20 chars wide minimum, exactly 3 rows tall (for border)
        Constraints {
            min_width: 20,
            max_width: None, // Can grow horizontally
            min_height: 3,
            max_height: Some(3), // Fixed height
            flex: None,          // Don't expand
        }
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

/// Create a new text input widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message { TextChanged(String) }
///
/// let input = text_input()
///     .placeholder("Enter text...")
///     .on_change(|text| Message::TextChanged(text));
/// ```
pub fn text_input<M>() -> TextInput<M> {
    TextInput::new()
}
