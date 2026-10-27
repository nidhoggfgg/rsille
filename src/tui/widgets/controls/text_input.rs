//! TextInput widget — stateful text input using WidgetStore

use crate::tui::text::{
    cursor_scroll, floor_boundary, paint_editor_line, EditBuffer, TextMetrics, UnicodeSegmentation,
};

use crate::tui::animation::AnimationCtx;
use crate::tui::event::{Event, MouseButton, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::border_renderer;
use crate::tui::layout::Constraints;
use crate::tui::style::{BorderStyle, Style, Theme};
use crate::tui::widget::{EventCtx, EventPhase, Lifecycle, RenderCtx, StateCtx, Widget};
use crate::tui::widgets::VariantWidget;

/// Text input style variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextInputVariant {
    #[default]
    Default,
    Borderless,
    Password,
}

/// Persistent state for [`TextInput`], stored in the [`WidgetStore`](crate::tui::widget::WidgetStore).
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
    pub selection_anchor: Option<usize>,
    /// Horizontal viewport offset in terminal columns.
    pub horizontal_offset: usize,
    /// Current editing value, synchronized before input and updated on each edit.
    pub value: Option<String>,
}

/// Single-line text input field.
///
/// The `value` is controlled by the application state (passed in via `.value()`).
/// The cursor position is managed by the framework via [`WidgetStore`](crate::tui::widget::WidgetStore).
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
    fn animate(&self, ctx: &mut AnimationCtx) -> bool {
        ctx.track_value(
            "input-hover",
            if !self.disabled && ctx.is_hovered() {
                1.0
            } else {
                0.0
            },
            ctx.animation_theme().fast,
        )
    }

    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() == Lifecycle::Unmount {
            return;
        }
        let state = ctx.state_mut::<TextInputState>();
        if state.value.as_deref() != Some(self.value.as_str()) {
            state.value = Some(self.value.clone());
        }
        state.cursor_position = floor_boundary(&self.value, state.cursor_position);
        state.selection_anchor = state
            .selection_anchor
            .map(|a| floor_boundary(&self.value, a));
    }

    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        ctx.record_bounds(area);
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
            let hover = if self.disabled {
                0.0
            } else {
                ctx.animation_value("input-hover")
                    .unwrap_or(if ctx.is_hovered() { 1.0 } else { 0.0 })
            };
            theme
                .styles
                .border
                .interpolate(theme.styles.border_focused, hover)
                .to_render_style()
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

        let state = ctx.state_or_default::<TextInputState>();
        let cursor = floor_boundary(&self.value, state.cursor_position);
        let metrics = theme.text_metrics;
        let mut selection = state
            .selection_anchor
            .filter(|a| *a != cursor)
            .map(|a| a.min(cursor)..a.max(cursor));
        let masked;
        let (text, cursor) = if self.variant == TextInputVariant::Password {
            selection = selection.map(|s| {
                self.value[..s.start].graphemes(true).count() * "•".len()
                    ..self.value[..s.end].graphemes(true).count() * "•".len()
            });
            masked = "•".repeat(self.value.graphemes(true).count());
            (
                masked.as_str(),
                self.value[..cursor].graphemes(true).count() * "•".len(),
            )
        } else {
            (self.value.as_str(), cursor)
        };
        let column = metrics.byte_to_column(text, cursor);
        let offset = cursor_scroll(column, state.horizontal_offset, available_width);

        if text.is_empty() && !is_focused {
            if let Some(placeholder) = &self.placeholder {
                let display = metrics.clip(placeholder, 0, available_width);
                let _ = chunk.set_string(text_start_x, text_y, &display, placeholder_style);
            }
        } else {
            paint_editor_line(
                chunk,
                (text_start_x, text_y),
                text,
                0,
                selection,
                (offset, available_width),
                render_style,
                theme.styles.selection_range.to_render_style(),
            );
            if is_focused {
                let g = text[cursor..].graphemes(true).next().unwrap_or(" ");
                let width = metrics.grapheme_width(g, column);
                let g = if width == 0 || column - offset + width > available_width {
                    " "
                } else {
                    g
                };
                let _ = chunk.set_forced(
                    text_start_x + (column - offset) as u16,
                    text_y,
                    crate::render::style::Stylized::grapheme(g, cursor_style),
                );
            }
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::tui::event::is_key_release(event) {
            return;
        }
        if ctx.phase() != EventPhase::Target || self.disabled {
            return;
        }
        let metrics = ctx.text_metrics();
        let local = ctx.local_mouse_position(event);
        let width = ctx.bounds().map(|a| a.width()).unwrap_or(24);
        let (start_x, available) = match self.variant {
            TextInputVariant::Borderless => (1, width.saturating_sub(2) as usize),
            _ => (2, width.saturating_sub(4) as usize),
        };
        let state = ctx.state_mut::<TextInputState>();
        let mut editor = EditBuffer::new(
            state.value.clone().unwrap_or_else(|| self.value.clone()),
            state.cursor_position,
            state.selection_anchor,
        );
        if let (Event::Mouse(mouse), Some((x, y))) = (event, local) {
            if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                && y == 1
                && x >= start_x
                && (x - start_x) < available as u16
            {
                let column = if self.variant == TextInputVariant::Password {
                    editor.value[..editor.cursor].graphemes(true).count() * metrics.width("•")
                } else {
                    metrics.byte_to_column(&editor.value, editor.cursor)
                };
                let offset = cursor_scroll(column, state.horizontal_offset, available);
                let column = offset + (x - start_x) as usize;
                editor.cursor = if self.variant == TextInputVariant::Password {
                    editor
                        .value
                        .grapheme_indices(true)
                        .nth(column / metrics.width("•").max(1))
                        .map(|(b, _)| b)
                        .unwrap_or(editor.value.len())
                } else {
                    metrics.column_to_byte(&editor.value, column)
                };
                editor.anchor = None;
            } else {
                return;
            }
        } else {
            let result = editor.handle(event, metrics, false);
            if !result.handled {
                return;
            }
            let message = if result.changed {
                self.on_change.as_ref().map(|f| f(editor.value.clone()))
            } else if result.submit {
                self.on_submit.as_ref().map(|f| f(editor.value.clone()))
            } else {
                None
            };
            state.cursor_position = editor.cursor;
            state.selection_anchor = editor.anchor;
            state.horizontal_offset = input_scroll(
                &editor.value,
                editor.cursor,
                state.horizontal_offset,
                available,
                metrics,
                self.variant,
            );
            state.value = Some(editor.value);
            ctx.set_handled();
            if let Some(message) = message {
                ctx.emit(message);
            }
            return;
        }
        state.cursor_position = editor.cursor;
        state.selection_anchor = editor.anchor;
        state.value = Some(editor.value);
        ctx.set_handled();
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

fn input_scroll(
    value: &str,
    cursor: usize,
    offset: usize,
    width: usize,
    metrics: TextMetrics,
    variant: TextInputVariant,
) -> usize {
    let column = if variant == TextInputVariant::Password {
        value[..cursor].graphemes(true).count() * metrics.width("•")
    } else {
        metrics.byte_to_column(value, cursor)
    };
    cursor_scroll(column, offset, width)
}
