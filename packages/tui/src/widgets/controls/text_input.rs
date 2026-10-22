//! TextInput widget — stateful text input using WidgetStore

use std::borrow::Cow;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::focus::FocusConfig;
use crate::layout::border_renderer;
use crate::layout::Constraints;
use crate::style::{BorderStyle, Style, Theme};
use crate::widget::{EventCtx, EventPhase, Lifecycle, RenderCtx, StateCtx, Widget};
use crate::widgets::VariantWidget;

/// Text input style variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextInputVariant {
    #[default]
    Default,
    Borderless,
    Password,
}

/// Persistent state for [`TextInput`], stored in the [`WidgetStore`].
///
/// This state survives `view()` rebuilds because it lives in the store,
/// not inside the widget instance.
///
/// Stores `value` for correct handling of key repeat: when multiple events
/// arrive in one frame (e.g. holding Backspace), we use the value from the
/// previous emit in the same batch instead of the stale `self.value` from
/// the cached tree.
#[derive(Debug, Clone, Default)]
pub struct TextInputState {
    pub cursor_position: usize,
    /// Current editing value, synchronized before input and updated on each edit.
    pub value: Option<String>,
}

/// Single-line text input field.
///
/// The `value` is controlled by the application state (passed in via `.value()`).
/// The cursor position is managed by the framework via [`WidgetStore`].
pub struct TextInput<M = ()> {
    value: String,
    placeholder: Option<String>,
    variant: TextInputVariant,
    disabled: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(String) -> M>>,
    on_submit: Option<Box<dyn Fn(String) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for TextInput<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInput")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.is_some())
            .field("on_submit", &self.on_submit.is_some())
            .finish()
    }
}

impl<M> TextInput<M> {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: None,
            variant: TextInputVariant::default(),
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
            if let Some(ref focus_style) = self.custom_focus_style {
                return focus_style.merge(base_style);
            }
        }

        self.custom_style
            .as_ref()
            .map(|s| s.merge(base_style))
            .unwrap_or(base_style)
    }
}

impl<M> VariantWidget for TextInput<M> {
    type Variant = TextInputVariant;

    fn variant(mut self, variant: Self::Variant) -> Self {
        self.variant = variant;
        self
    }
}

impl<M> Default for TextInput<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for TextInput<M> {
    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() == Lifecycle::Unmount {
            return;
        }
        let state = ctx.state_mut::<TextInputState>();
        if state.value.as_deref() != Some(self.value.as_str()) {
            state.value = Some(self.value.clone());
        }
        state.cursor_position = state.cursor_position.min(self.value.len());
        while !self.value.is_char_boundary(state.cursor_position) {
            state.cursor_position -= 1;
        }
    }

    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        let width = area.width();
        let height = area.height();

        if width < 4 || height < 3 {
            return;
        }

        let is_focused = ctx.is_focused();
        let theme = ctx.theme();
        let style = self.compute_style(theme, is_focused);
        let render_style = style.to_render_style();

        // Single theme lookup for all theme-dependent styles
        let border_style = if is_focused {
            theme.styles.border_focused.to_render_style()
        } else {
            theme.styles.border.to_render_style()
        };
        let placeholder_style = theme.styles.text_placeholder.to_render_style();
        let cursor_style = theme.styles.cursor.to_render_style();

        match self.variant {
            TextInputVariant::Default | TextInputVariant::Password => {
                border_renderer::render_border(chunk, BorderStyle::Single, border_style);
            }
            TextInputVariant::Borderless => {
                border_renderer::render_border_bottom(chunk, BorderStyle::Single, border_style);
            }
        }

        let (text_y, text_start_x, available_width) = match self.variant {
            TextInputVariant::Borderless => (1, 1u16, (width - 2) as usize),
            _ => (1, 2u16, (width - 4) as usize),
        };

        if available_width == 0 {
            return;
        }

        // Read cursor position from the persistent store
        let cursor_position = ctx
            .state::<TextInputState>()
            .map(|s| s.cursor_position.min(self.value.len()))
            .unwrap_or_else(|| self.value.len());

        if self.value.is_empty() {
            if !is_focused {
                if let Some(ref placeholder) = self.placeholder {
                    let display_text: String = placeholder.chars().take(available_width).collect();
                    let _ =
                        chunk.set_string(text_start_x, text_y, &display_text, placeholder_style);
                }
            } else {
                let _ = chunk.set_char(text_start_x, text_y, ' ', cursor_style);
            }
        } else {
            let text_before_cursor = &self.value[..cursor_position];
            let cursor_visual_pos = text_before_cursor.width();

            // Build display text: avoid clone when no truncation needed
            let display_text: Cow<str> = if self.variant == TextInputVariant::Password {
                let masked = "•".repeat(self.value.chars().count());
                if masked.width() > available_width {
                    let mut result = String::new();
                    let mut w = 0;
                    for ch in masked.chars() {
                        let ch_w = ch.width().unwrap_or(0);
                        if w + ch_w > available_width {
                            break;
                        }
                        result.push(ch);
                        w += ch_w;
                    }
                    Cow::Owned(result)
                } else {
                    Cow::Owned(masked)
                }
            } else if self.value.width() > available_width {
                let mut result = String::new();
                let mut w = 0;
                for ch in self.value.chars() {
                    let ch_w = ch.width().unwrap_or(0);
                    if w + ch_w > available_width {
                        break;
                    }
                    result.push(ch);
                    w += ch_w;
                }
                Cow::Owned(result)
            } else {
                Cow::Borrowed(self.value.as_str())
            };

            let _ = chunk.set_string(text_start_x, text_y, display_text.as_ref(), render_style);

            if is_focused && cursor_visual_pos <= available_width {
                let cursor_x = text_start_x + cursor_visual_pos as u16;
                let cursor_char = if cursor_position >= self.value.len() {
                    ' '
                } else if self.variant == TextInputVariant::Password {
                    '•'
                } else {
                    self.value[cursor_position..].chars().next().unwrap_or(' ')
                };
                let _ = chunk.set_char(cursor_x, text_y, cursor_char, cursor_style);
            }
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target {
            return;
        }
        if self.disabled {
            return;
        }

        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event
        {
            let mut msg_to_emit: Option<M> = None;

            {
                let state = ctx.state_mut::<TextInputState>();

                // Fallback for manually constructed EventCtx; runtime syncs before input.
                if state.value.is_none() {
                    state.value = Some(self.value.clone());
                }

                let value: &str = state.value.as_deref().unwrap_or(self.value.as_str());
                let value_len = value.len();

                // Clamp cursor to value length
                state.cursor_position = state.cursor_position.min(value_len);

                match code {
                    KeyCode::Char(c) => {
                        if modifiers.contains(KeyModifiers::CONTROL) {
                            if *c == 'a' {
                                state.cursor_position = value_len;
                            }
                            return;
                        }
                        let mut new_val = value.to_string();
                        new_val.insert(state.cursor_position, *c);
                        state.cursor_position += c.len_utf8();
                        state.value = Some(new_val.clone());
                        if let Some(ref handler) = self.on_change {
                            msg_to_emit = Some(handler(new_val));
                        }
                    }
                    KeyCode::Backspace => {
                        if state.cursor_position > 0 {
                            let mut idx = state.cursor_position - 1;
                            while idx > 0 && !value.is_char_boundary(idx) {
                                idx -= 1;
                            }
                            let mut new_val = value.to_string();
                            new_val.remove(idx);
                            state.cursor_position = idx;
                            state.value = Some(new_val.clone());
                            if let Some(ref handler) = self.on_change {
                                msg_to_emit = Some(handler(new_val));
                            }
                        }
                    }
                    KeyCode::Delete => {
                        if state.cursor_position < value_len {
                            // Ensure we're at a char boundary for multi-byte UTF-8
                            let mut idx = state.cursor_position;
                            while idx < value_len && !value.is_char_boundary(idx) {
                                idx += 1;
                            }
                            if idx < value_len {
                                let mut new_val = value.to_string();
                                new_val.remove(idx);
                                state.value = Some(new_val.clone());
                                if let Some(ref handler) = self.on_change {
                                    msg_to_emit = Some(handler(new_val));
                                }
                            }
                        }
                    }
                    KeyCode::Left => {
                        if state.cursor_position > 0 {
                            let mut idx = state.cursor_position - 1;
                            while idx > 0 && !value.is_char_boundary(idx) {
                                idx -= 1;
                            }
                            state.cursor_position = idx;
                        }
                    }
                    KeyCode::Right => {
                        if state.cursor_position < value_len {
                            let mut idx = state.cursor_position + 1;
                            while idx < value_len && !value.is_char_boundary(idx) {
                                idx += 1;
                            }
                            state.cursor_position = idx;
                        }
                    }
                    KeyCode::Home => {
                        state.cursor_position = 0;
                    }
                    KeyCode::End => {
                        state.cursor_position = value_len;
                    }
                    KeyCode::Enter => {
                        if let Some(ref handler) = self.on_submit {
                            msg_to_emit = Some(handler(value.to_string()));
                        }
                    }
                    _ => {}
                }
            }

            if let Some(msg) = msg_to_emit {
                ctx.emit(msg);
            }
        }
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: 20,
            max_width: None,
            min_height: 3,
            max_height: Some(3),
            flex: None,
        }
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

/// Create a new text input widget.
pub fn text_input<M>() -> TextInput<M> {
    TextInput::new()
}
