//! Textarea widget - multi-line text input component

use super::*;
use crate::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::layout::border_renderer;
use crate::style::{BorderStyle, Style, ThemeManager};
use std::sync::Arc;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Textarea style variants
///
/// Different visual styles for textareas based on their purpose and context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextareaVariant {
    /// Default bordered textarea
    #[default]
    Default,
    /// Borderless textarea (bottom border only)
    Borderless,
}

/// Represents a line selection range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Start position (byte offset)
    pub start: usize,
    /// End position (byte offset)
    pub end: usize,
}

impl Selection {
    /// Create a new selection
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Check if selection is empty
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Get normalized selection (start < end)
    pub fn normalized(&self) -> (usize, usize) {
        if self.start <= self.end {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }
}

/// Textarea widget
///
/// A multi-line text input field with support for editing, cursor navigation,
/// line wrapping, scrolling, and optional line numbers.
///
/// # Examples
/// ```
/// use tui::widget::{Textarea, TextareaVariant};
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     TextChanged(String),
///     Submit,
/// }
///
/// let textarea = Textarea::new()
///     .variant(TextareaVariant::Default)
///     .placeholder("Enter your code here...")
///     .value("fn main() {\n    println!(\"Hello\");\n}")
///     .line_numbers(true)
///     .on_change(|text| Message::TextChanged(text))
///     .on_submit(|text| Message::Submit);
/// ```
#[derive(Clone)]
pub struct Textarea<M = ()> {
    // Content stored as single string with \n line separators
    value: String,
    placeholder: Option<String>,

    // Cursor state (byte offset in the entire text)
    cursor_position: usize,

    // Selection state
    selection: Option<Selection>,

    // Scroll state
    scroll_offset: usize, // Number of lines scrolled from top

    // Configuration
    variant: TextareaVariant,
    disabled: bool,
    line_numbers: bool,
    line_wrap: bool,

    // State
    focused: bool,

    // Styling
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,

    // Event handlers
    on_change: Option<Arc<dyn Fn(String) -> M + Send + Sync>>,
    on_submit: Option<Arc<dyn Fn(String) -> M + Send + Sync>>,
}

impl<M> std::fmt::Debug for Textarea<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Textarea")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("cursor_position", &self.cursor_position)
            .field("selection", &self.selection)
            .field("scroll_offset", &self.scroll_offset)
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .field("line_numbers", &self.line_numbers)
            .field("line_wrap", &self.line_wrap)
            .field("focused", &self.focused)
            .field("custom_style", &self.custom_style)
            .field("custom_focus_style", &self.custom_focus_style)
            .field("on_change", &self.on_change.is_some())
            .field("on_submit", &self.on_submit.is_some())
            .finish()
    }
}

impl<M> Textarea<M> {
    /// Create a new textarea
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Textarea;
    ///
    /// let textarea = Textarea::<()>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: None,
            cursor_position: 0,
            selection: None,
            scroll_offset: 0,
            variant: TextareaVariant::default(),
            disabled: false,
            line_numbers: false,
            line_wrap: true,
            focused: false,
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            on_submit: None,
        }
    }

    /// Set the textarea variant
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{Textarea, TextareaVariant};
    ///
    /// let textarea = Textarea::<()>::new()
    ///     .variant(TextareaVariant::Borderless);
    /// ```
    pub fn variant(mut self, variant: TextareaVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the textarea value
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Textarea;
    ///
    /// let textarea = Textarea::<()>::new()
    ///     .value("Hello\nWorld");
    /// ```
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        // Move cursor to end of value
        self.cursor_position = self.value.len();
        self
    }

    /// Set the placeholder text
    ///
    /// Placeholder is shown when the textarea is empty.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Textarea;
    ///
    /// let textarea = Textarea::<()>::new()
    ///     .placeholder("Enter text...");
    /// ```
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the disabled state
    ///
    /// Disabled textareas cannot be interacted with.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Textarea;
    ///
    /// let textarea = Textarea::<()>::new()
    ///     .disabled(true);
    /// ```
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Enable or disable line numbers
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Textarea;
    ///
    /// let textarea = Textarea::<()>::new()
    ///     .line_numbers(true);
    /// ```
    pub fn line_numbers(mut self, enabled: bool) -> Self {
        self.line_numbers = enabled;
        self
    }

    /// Enable or disable line wrapping
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Textarea;
    ///
    /// let textarea = Textarea::<()>::new()
    ///     .line_wrap(false);
    /// ```
    pub fn line_wrap(mut self, enabled: bool) -> Self {
        self.line_wrap = enabled;
        self
    }

    /// Set a custom style
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Textarea;
    /// use tui::style::{Style, Color};
    ///
    /// let textarea = Textarea::<()>::new()
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
    /// use tui::widget::Textarea;
    /// use tui::style::{Style, Color};
    ///
    /// let textarea = Textarea::<()>::new()
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
    /// use tui::widget::Textarea;
    ///
    /// #[derive(Clone)]
    /// enum Message { TextChanged(String) }
    ///
    /// let textarea = Textarea::new()
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
    /// Called when Ctrl+Enter is pressed.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Textarea;
    ///
    /// #[derive(Clone)]
    /// enum Message { Submit(String) }
    ///
    /// let textarea = Textarea::new()
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

        // If custom focus style is provided and textarea is focused, use it
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

    /// Get lines from value
    fn lines(&self) -> Vec<&str> {
        if self.value.is_empty() {
            vec![]
        } else {
            self.value.split('\n').collect()
        }
    }

    /// Get cursor line and column
    fn cursor_line_col(&self) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        for (idx, ch) in self.value.char_indices() {
            if idx >= self.cursor_position {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += ch.width().unwrap_or(0);
            }
        }
        (line, col)
    }

    /// Convert line and column to byte offset
    fn line_col_to_offset(&self, target_line: usize, target_col: usize) -> usize {
        let mut line = 0;
        let mut col = 0;
        for (idx, ch) in self.value.char_indices() {
            if line == target_line && col >= target_col {
                return idx;
            }
            if ch == '\n' {
                if line == target_line {
                    return idx; // End of target line
                }
                line += 1;
                col = 0;
            } else {
                col += ch.width().unwrap_or(0);
            }
        }
        self.value.len()
    }

    /// Insert a character at the cursor position
    fn insert_char(&mut self, c: char) -> Vec<M> {
        // Clear selection if exists
        if let Some(sel) = self.selection.take() {
            self.delete_selection(sel);
        }

        self.value.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
        self.emit_change()
    }

    /// Insert a newline at the cursor position
    fn insert_newline(&mut self) -> Vec<M> {
        // Clear selection if exists
        if let Some(sel) = self.selection.take() {
            self.delete_selection(sel);
        }

        self.value.insert(self.cursor_position, '\n');
        self.cursor_position += 1;
        self.emit_change()
    }

    /// Delete character before cursor (Backspace)
    fn delete_before_cursor(&mut self) -> Vec<M> {
        // If there's a selection, delete it
        if let Some(sel) = self.selection.take() {
            return self.delete_selection(sel);
        }

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
        // If there's a selection, delete it
        if let Some(sel) = self.selection.take() {
            return self.delete_selection(sel);
        }

        if self.cursor_position < self.value.len() {
            self.value.remove(self.cursor_position);
            self.emit_change()
        } else {
            Vec::new()
        }
    }

    /// Delete the selected text
    fn delete_selection(&mut self, selection: Selection) -> Vec<M> {
        let (start, end) = selection.normalized();
        self.value.drain(start..end);
        self.cursor_position = start;
        self.emit_change()
    }

    /// Move cursor up
    fn move_cursor_up(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line > 0 {
            self.cursor_position = self.line_col_to_offset(line - 1, col);
        }
    }

    /// Move cursor down
    fn move_cursor_down(&mut self) {
        let (line, col) = self.cursor_line_col();
        let lines = self.lines();
        if line < lines.len().saturating_sub(1) {
            self.cursor_position = self.line_col_to_offset(line + 1, col);
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

    /// Move cursor to start of line
    fn move_cursor_home(&mut self) {
        let (line, _) = self.cursor_line_col();
        self.cursor_position = self.line_col_to_offset(line, 0);
    }

    /// Move cursor to end of line
    fn move_cursor_end(&mut self) {
        let (line, _) = self.cursor_line_col();
        let lines = self.lines();
        if line < lines.len() {
            let line_text = lines[line];
            self.cursor_position = self.line_col_to_offset(line, line_text.width());
        }
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

}

impl<M> Default for Textarea<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Send + Sync> Widget<M> for Textarea<M> {
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
            TextareaVariant::Default => {
                // Render full border
                border_renderer::render_border(chunk, BorderStyle::Single, border_style);
            }
            TextareaVariant::Borderless => {
                // Render only bottom border
                border_renderer::render_border_bottom(chunk, BorderStyle::Single, border_style);
            }
        }

        // Calculate text area
        let (text_y, text_start_x, available_width, available_height) = match self.variant {
            TextareaVariant::Borderless => {
                let text_y = 0;
                let text_start_x = 1u16;
                let available_width = (width - 2) as usize;
                let available_height = (height - 1) as usize;
                (text_y, text_start_x, available_width, available_height)
            }
            _ => {
                let text_y = 1;
                let text_start_x = 2u16;
                let available_width = (width - 4) as usize;
                let available_height = (height - 2) as usize;
                (text_y, text_start_x, available_width, available_height)
            }
        };

        if available_width == 0 || available_height == 0 {
            return;
        }

        // Calculate line number width if enabled
        let line_num_width = if self.line_numbers {
            let lines = self.lines();
            let num_lines = lines.len().max(1);
            let width = format!("{}", num_lines).len();
            width + 1 // Add space after line number
        } else {
            0
        };

        let text_content_x = text_start_x + line_num_width as u16;
        let text_content_width = available_width.saturating_sub(line_num_width);

        if text_content_width == 0 {
            return;
        }

        // Show placeholder if empty and not focused
        if self.value.is_empty() && !self.focused {
            if let Some(ref placeholder) = self.placeholder {
                let placeholder_style = ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.text_muted)
                        .to_render_style()
                });

                let display_text: String = placeholder.chars().take(text_content_width).collect();
                let _ = chunk.set_string(text_content_x, text_y, &display_text, placeholder_style);
            }

            // Show cursor at start when focused
            if self.focused {
                let cursor_style = ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.background)
                        .bg(theme.colors.text)
                        .to_render_style()
                });
                let _ = chunk.set_char(text_content_x, text_y, ' ', cursor_style);
            }
            return;
        }

        // Render text content
        let lines = self.lines();
        let (cursor_line, cursor_col) = self.cursor_line_col();

        // Calculate scroll offset to ensure cursor is visible
        let scroll_offset = if cursor_line < self.scroll_offset {
            // Cursor is above visible area, scroll up
            cursor_line
        } else if cursor_line >= self.scroll_offset + available_height {
            // Cursor is below visible area, scroll down
            cursor_line.saturating_sub(available_height - 1)
        } else {
            // Cursor is in visible area, keep current scroll
            self.scroll_offset
        };

        let visible_start = scroll_offset;
        let visible_end = (scroll_offset + available_height).min(lines.len());

        for (display_idx, line_idx) in (visible_start..visible_end).enumerate() {
            let y = text_y + display_idx as u16;
            let line_text = lines.get(line_idx).unwrap_or(&"");

            // Render line number if enabled
            if self.line_numbers {
                let line_num_style = ThemeManager::global().with_theme(|theme| {
                    Style::default()
                        .fg(theme.colors.text_muted)
                        .to_render_style()
                });
                let line_num = format!("{:>width$} ", line_idx + 1, width = line_num_width - 1);
                let _ = chunk.set_string(text_start_x, y, &line_num, line_num_style);
            }

            // Render line text
            let display_text = if self.line_wrap {
                // Truncate if too long
                let mut result = String::new();
                let mut w = 0;
                for ch in line_text.chars() {
                    let ch_w = ch.width().unwrap_or(0);
                    if w + ch_w > text_content_width {
                        break;
                    }
                    result.push(ch);
                    w += ch_w;
                }
                result
            } else {
                line_text.to_string()
            };

            let _ = chunk.set_string(text_content_x, y, &display_text, render_style);

            // Render cursor if on this line and focused
            if self.focused && line_idx == cursor_line {
                let cursor_x = text_content_x + cursor_col as u16;

                if cursor_col < text_content_width {
                    let cursor_style = ThemeManager::global().with_theme(|theme| {
                        Style::default()
                            .fg(theme.colors.background)
                            .bg(theme.colors.text)
                            .to_render_style()
                    });

                    // Find character at cursor position
                    let cursor_char = if cursor_col >= line_text.width() {
                        ' ' // Block cursor at end of line
                    } else {
                        let mut current_width = 0;
                        line_text
                            .chars()
                            .find(|&ch| {
                                let ch_w = ch.width().unwrap_or(0);
                                if current_width == cursor_col {
                                    true
                                } else {
                                    current_width += ch_w;
                                    false
                                }
                            })
                            .unwrap_or(' ')
                    };

                    let _ = chunk.set_char(cursor_x, y, cursor_char, cursor_style);
                }
            }
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // Disabled textareas don't handle events
        if self.disabled {
            return EventResult::Ignored;
        }

        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event
        {
            match code {
                // Character input
                KeyCode::Char(c) => {
                    // Check for Ctrl combinations
                    if modifiers.contains(KeyModifiers::CONTROL) {
                        match c {
                            'a' => {
                                // Select all
                                if !self.value.is_empty() {
                                    self.selection = Some(Selection::new(0, self.value.len()));
                                }
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

                // Enter - insert newline (unless Ctrl+Enter for submit)
                KeyCode::Enter => {
                    if modifiers.contains(KeyModifiers::CONTROL) {
                        // Ctrl+Enter - submit
                        let messages = self.emit_submit();
                        return EventResult::Consumed(messages);
                    } else {
                        // Regular Enter - newline
                        let messages = self.insert_newline();
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
                KeyCode::Up => {
                    self.selection = None; // Clear selection
                    self.move_cursor_up();
                    return EventResult::Consumed(Vec::new());
                }
                KeyCode::Down => {
                    self.selection = None; // Clear selection
                    self.move_cursor_down();
                    return EventResult::Consumed(Vec::new());
                }
                KeyCode::Left => {
                    self.selection = None; // Clear selection
                    self.move_cursor_left();
                    return EventResult::Consumed(Vec::new());
                }
                KeyCode::Right => {
                    self.selection = None; // Clear selection
                    self.move_cursor_right();
                    return EventResult::Consumed(Vec::new());
                }

                // Home/End
                KeyCode::Home => {
                    self.selection = None; // Clear selection
                    self.move_cursor_home();
                    return EventResult::Consumed(Vec::new());
                }
                KeyCode::End => {
                    self.selection = None; // Clear selection
                    self.move_cursor_end();
                    return EventResult::Consumed(Vec::new());
                }

                _ => {}
            }
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        // Flexible size: minimum 20 chars wide, minimum 5 rows tall
        Constraints {
            min_width: 20,
            max_width: None, // Can grow horizontally
            min_height: 5,
            max_height: None, // Can grow vertically
            flex: None,
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
        // Clear selection when losing focus
        if !focused {
            self.selection = None;
        }
    }
}

/// Create a new textarea widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message { TextChanged(String) }
///
/// let textarea = textarea()
///     .placeholder("Enter text...")
///     .line_numbers(true)
///     .on_change(|text| Message::TextChanged(text));
/// ```
pub fn textarea<M>() -> Textarea<M> {
    Textarea::new()
}
