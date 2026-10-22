//! Multiline text area widget.

use std::borrow::Cow;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::focus::FocusConfig;
use crate::layout::border_renderer;
use crate::layout::{ensure_item_visible, Constraints, LayoutStyle};
use crate::style::{BorderStyle, Style, Theme};
use crate::widget::{EventCtx, EventPhase, Lifecycle, RenderCtx, StateCtx, Widget};
use crate::widgets::VariantWidget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAreaVariant {
    #[default]
    Default,
    Borderless,
}

#[derive(Debug, Clone, Default)]
pub struct TextAreaState {
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub value: Option<String>,
}

pub struct TextArea<M = ()> {
    value: String,
    placeholder: Option<String>,
    height: u16,
    variant: TextAreaVariant,
    disabled: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(String) -> M>>,
    on_submit: Option<Box<dyn Fn(String) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for TextArea<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextArea")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("height", &self.height)
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.is_some())
            .field("on_submit", &self.on_submit.is_some())
            .finish()
    }
}

impl<M> TextArea<M> {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: None,
            height: 5,
            variant: TextAreaVariant::default(),
            disabled: false,
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            on_submit: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height.max(3);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_change = Some(Box::new(handler));
        self
    }

    pub fn on_submit<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_submit = Some(Box::new(handler));
        self
    }

    fn compute_style(&self, theme: &Theme, is_focused: bool) -> Style {
        let base_style = if self.disabled {
            theme.styles.interactive_disabled
        } else if is_focused {
            theme.styles.interactive_focused
        } else {
            theme.styles.interactive
        };

        if is_focused {
            if let Some(focus_style) = self.custom_focus_style {
                return focus_style.merge(base_style);
            }
        }

        self.custom_style
            .map(|style| style.merge(base_style))
            .unwrap_or(base_style)
    }
}

impl<M> VariantWidget for TextArea<M> {
    type Variant = TextAreaVariant;

    fn variant(mut self, variant: Self::Variant) -> Self {
        self.variant = variant;
        self
    }
}

impl<M> Default for TextArea<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for TextArea<M> {
    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() == Lifecycle::Unmount {
            return;
        }
        let state = ctx.state_mut::<TextAreaState>();
        if state.value.as_deref() != Some(self.value.as_str()) {
            state.value = Some(self.value.clone());
        }
        state.cursor_position = state.cursor_position.min(self.value.len());
        while !self.value.is_char_boundary(state.cursor_position) {
            state.cursor_position -= 1;
        }
        let (line, _) = line_col_for_byte(&self.value, state.cursor_position);
        let visible_rows = match self.variant {
            TextAreaVariant::Default => self.height.saturating_sub(2),
            TextAreaVariant::Borderless => self.height,
        };
        state.scroll_offset =
            ensure_item_visible(state.scroll_offset, line, visible_rows.max(1) as usize);
    }

    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let is_focused = ctx.is_focused();
        let theme = ctx.theme();
        let render_style = self.compute_style(theme, is_focused).to_render_style();
        let border_style = if is_focused {
            theme.styles.border_focused.to_render_style()
        } else {
            theme.styles.border.to_render_style()
        };
        let placeholder_style = theme.styles.text_placeholder.to_render_style();
        let cursor_style = theme.styles.cursor.to_render_style();

        let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', render_style);
        let (content_x, content_y, content_width, content_height) = match self.variant {
            TextAreaVariant::Default => {
                if area.width() < 2 || area.height() < 2 {
                    return;
                }
                border_renderer::render_border(chunk, BorderStyle::Single, border_style);
                (1u16, 1u16, area.width() - 2, area.height() - 2)
            }
            TextAreaVariant::Borderless => (0u16, 0u16, area.width(), area.height()),
        };

        if content_width == 0 || content_height == 0 {
            return;
        }

        let state = ctx.state_or_default::<TextAreaState>();
        let cursor_position = state.cursor_position.min(self.value.len());
        let (cursor_line, cursor_col) = line_col_for_byte(&self.value, cursor_position);
        let visible_rows = content_height as usize;
        let scroll_offset = ensure_item_visible(state.scroll_offset, cursor_line, visible_rows);

        if self.value.is_empty() {
            if let Some(placeholder) = self.placeholder.as_ref() {
                if !is_focused {
                    let display = truncate_to_width(placeholder, content_width as usize);
                    let _ = chunk.set_string(content_x, content_y, &display, placeholder_style);
                }
            }
            if is_focused {
                let _ = chunk.set_char(content_x, content_y, ' ', cursor_style);
            }
            return;
        }

        for (row, line) in self
            .value
            .split('\n')
            .skip(scroll_offset)
            .take(visible_rows)
            .enumerate()
        {
            let display: Cow<str> = if line.width() > content_width as usize {
                Cow::Owned(truncate_to_width(line, content_width as usize))
            } else {
                Cow::Borrowed(line)
            };
            let _ = chunk.set_string(content_x, content_y + row as u16, &display, render_style);
        }

        if is_focused
            && cursor_line >= scroll_offset
            && cursor_line < scroll_offset + visible_rows
            && cursor_col <= content_width as usize
        {
            let y = content_y + (cursor_line - scroll_offset) as u16;
            let cursor_line_text = self.value.split('\n').nth(cursor_line).unwrap_or("");
            let before_cursor = take_chars(cursor_line_text, cursor_col);
            let cursor_x = content_x + before_cursor.width().min(content_width as usize) as u16;
            if cursor_x < content_x + content_width {
                let cursor_char = cursor_line_text.chars().nth(cursor_col).unwrap_or(' ');
                let _ = chunk.set_char(cursor_x, y, cursor_char, cursor_style);
            }
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target || self.disabled {
            return;
        }

        let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event
        else {
            return;
        };

        let mut msg_to_emit = None;
        {
            let state = ctx.state_mut::<TextAreaState>();
            // Fallback for manually constructed EventCtx; runtime syncs before input.
            if state.value.is_none() {
                state.value = Some(self.value.clone());
            }

            let value = state.value.as_deref().unwrap_or(self.value.as_str());
            state.cursor_position = state.cursor_position.min(value.len());
            let mut new_cursor = state.cursor_position;
            let mut next_value = None;

            match code {
                KeyCode::Char(c) => {
                    if modifiers.contains(KeyModifiers::CONTROL) {
                        return;
                    }
                    let mut updated = value.to_string();
                    updated.insert(new_cursor, *c);
                    new_cursor += c.len_utf8();
                    next_value = Some(updated);
                }
                KeyCode::Enter => {
                    if modifiers.contains(KeyModifiers::CONTROL) {
                        if let Some(handler) = self.on_submit.as_ref() {
                            msg_to_emit = Some(handler(value.to_string()));
                        }
                    } else {
                        let mut updated = value.to_string();
                        updated.insert(new_cursor, '\n');
                        new_cursor += 1;
                        next_value = Some(updated);
                    }
                }
                KeyCode::Backspace => {
                    if new_cursor > 0 {
                        let previous = previous_char_boundary(value, new_cursor);
                        let mut updated = value.to_string();
                        updated.replace_range(previous..new_cursor, "");
                        new_cursor = previous;
                        next_value = Some(updated);
                    }
                }
                KeyCode::Delete => {
                    if new_cursor < value.len() {
                        let next = next_char_boundary(value, new_cursor);
                        let mut updated = value.to_string();
                        updated.replace_range(new_cursor..next, "");
                        next_value = Some(updated);
                    }
                }
                KeyCode::Left => new_cursor = previous_char_boundary(value, new_cursor),
                KeyCode::Right => new_cursor = next_char_boundary(value, new_cursor),
                KeyCode::Up => {
                    let (line, col) = line_col_for_byte(value, new_cursor);
                    if line > 0 {
                        new_cursor = byte_for_line_col(value, line - 1, col);
                    }
                }
                KeyCode::Down => {
                    let (line, col) = line_col_for_byte(value, new_cursor);
                    let last_line = value.split('\n').count().saturating_sub(1);
                    if line < last_line {
                        new_cursor = byte_for_line_col(value, line + 1, col);
                    }
                }
                KeyCode::Home => {
                    let (line, _) = line_col_for_byte(value, new_cursor);
                    new_cursor = byte_for_line_col(value, line, 0);
                }
                KeyCode::End => {
                    let (line, _) = line_col_for_byte(value, new_cursor);
                    let line_len = value
                        .split('\n')
                        .nth(line)
                        .map(str::chars)
                        .map(Iterator::count)
                        .unwrap_or(0);
                    new_cursor = byte_for_line_col(value, line, line_len);
                }
                _ => return,
            }

            if let Some(updated) = next_value {
                state.cursor_position = new_cursor;
                let (line, _) = line_col_for_byte(&updated, new_cursor);
                let visible_rows = self.height.saturating_sub(2) as usize;
                state.scroll_offset =
                    ensure_item_visible(state.scroll_offset, line, visible_rows.max(1));
                state.value = Some(updated.clone());
                if let Some(handler) = self.on_change.as_ref() {
                    msg_to_emit = Some(handler(updated));
                }
            } else {
                state.cursor_position = new_cursor;
                let (line, _) = line_col_for_byte(value, new_cursor);
                let visible_rows = self.height.saturating_sub(2) as usize;
                state.scroll_offset =
                    ensure_item_visible(state.scroll_offset, line, visible_rows.max(1));
            }
        }

        ctx.set_handled();
        if let Some(msg) = msg_to_emit {
            ctx.emit(msg);
        }
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: self.preferred_width(),
            max_width: None,
            min_height: self.height,
            max_height: Some(self.height),
            flex: Some(1.0),
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        let mut style = LayoutStyle::from_constraints(self.constraints());
        style.preferred_width = Some(self.preferred_width());
        style
    }

    fn focus_config(&self) -> FocusConfig {
        if self.disabled {
            FocusConfig::None
        } else {
            FocusConfig::Leaf
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

fn previous_char_boundary(value: &str, cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }
    let mut idx = cursor.saturating_sub(1);
    while idx > 0 && !value.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn next_char_boundary(value: &str, cursor: usize) -> usize {
    if cursor >= value.len() {
        return value.len();
    }
    let mut idx = cursor + 1;
    while idx < value.len() && !value.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

fn line_col_for_byte(value: &str, cursor: usize) -> (usize, usize) {
    let cursor = cursor.min(value.len());
    let mut line = 0;
    let mut col = 0;
    for (idx, ch) in value.char_indices() {
        if idx >= cursor {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn byte_for_line_col(value: &str, target_line: usize, target_col: usize) -> usize {
    let mut line = 0;
    let mut col = 0;
    for (idx, ch) in value.char_indices() {
        if line == target_line && col == target_col {
            return idx;
        }
        if ch == '\n' {
            if line == target_line {
                return idx;
            }
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    value.len()
}

fn take_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if width + ch_width > max_width {
            break;
        }
        out.push(ch);
        width += ch_width;
    }
    out
}

pub fn textarea<M>() -> TextArea<M> {
    TextArea::new()
}

impl<M> TextArea<M> {
    fn preferred_width(&self) -> u16 {
        let value_width = self
            .value
            .split('\n')
            .map(|line| line.width() as u16)
            .max()
            .unwrap_or(0);
        let placeholder_width = self
            .placeholder
            .as_deref()
            .map(|placeholder| placeholder.width() as u16)
            .unwrap_or(0);
        value_width.max(placeholder_width).max(20).saturating_add(
            if self.variant == TextAreaVariant::Default {
                2
            } else {
                0
            },
        )
    }
}
