//! Multiline text area widget.

use crate::tui::text::{
    cursor_scroll, floor_boundary, paint_editor_line, EditBuffer, TextMetrics, UnicodeSegmentation,
};

use crate::tui::animation::AnimationCtx;
use crate::tui::event::{Event, KeyModifiers, MouseButton, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::border_renderer;
use crate::tui::layout::{ensure_item_visible, Constraints, LayoutStyle};
use crate::tui::style::{BorderStyle, Style, Theme};
use crate::tui::widget::{EventCtx, EventPhase, Lifecycle, RenderCtx, StateCtx, Widget};
use crate::tui::widgets::pointer::{self, EditorPointer};
use crate::tui::widgets::VariantWidget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAreaVariant {
    #[default]
    Default,
    Borderless,
}

#[derive(Debug, Clone, Default)]
pub struct TextAreaState {
    /// Keep the wheel-scrolled viewport independent of the caret until editing resumes.
    pub viewport_scrolled: bool,
    /// Whether a left-button selection gesture is in progress.
    pub dragging: bool,
    pub cursor_position: usize,
    pub selection_anchor: Option<usize>,
    pub horizontal_offset: usize,
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
        let state = ctx.state_mut::<TextAreaState>();
        if state.value.as_deref() != Some(self.value.as_str()) {
            state.value = Some(self.value.clone());
            state.dragging = false;
            state.viewport_scrolled = false;
        }
        state.cursor_position = floor_boundary(&self.value, state.cursor_position);
        state.selection_anchor = state
            .selection_anchor
            .map(|a| floor_boundary(&self.value, a));
        let (line, _) =
            TextMetrics::default().byte_to_line_column(&self.value, state.cursor_position);
        let visible_rows = match self.variant {
            TextAreaVariant::Default => self.height.saturating_sub(2),
            TextAreaVariant::Borderless => self.height,
        };
        if !state.viewport_scrolled {
            state.scroll_offset =
                ensure_item_visible(state.scroll_offset, line, visible_rows.max(1) as usize);
        }
    }

    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        ctx.record_bounds(area);
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let is_focused = ctx.is_focused();
        let theme = ctx.theme();
        let render_style = self.compute_style(theme, is_focused).to_render_style();
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
        let cursor_position = floor_boundary(&self.value, state.cursor_position);
        let metrics = theme.text_metrics;
        let (cursor_line, cursor_col) = metrics.byte_to_line_column(&self.value, cursor_position);
        let horizontal_offset =
            cursor_scroll(cursor_col, state.horizontal_offset, content_width as usize);
        let visible_rows = content_height as usize;
        let scroll_offset = if state.viewport_scrolled {
            state
                .scroll_offset
                .min(self.value.split('\n').count().saturating_sub(visible_rows))
        } else {
            ensure_item_visible(state.scroll_offset, cursor_line, visible_rows)
        };

        if self.value.is_empty() {
            if let Some(placeholder) = self.placeholder.as_ref() {
                if !is_focused {
                    let display = metrics.clip(placeholder, 0, content_width as usize);
                    let _ = chunk.set_string(content_x, content_y, &display, placeholder_style);
                }
            }
            if is_focused {
                let _ = chunk.set_char(content_x, content_y, ' ', cursor_style);
            }
            return;
        }

        let selection = state
            .selection_anchor
            .filter(|a| *a != cursor_position)
            .map(|a| a.min(cursor_position)..a.max(cursor_position));
        for (row, (byte_start, line)) in self
            .value
            .split('\n')
            .scan(0, |offset, line| {
                let start = *offset;
                *offset += line.len() + 1;
                Some((start, line))
            })
            .skip(scroll_offset)
            .take(visible_rows)
            .enumerate()
        {
            paint_editor_line(
                chunk,
                (content_x, content_y + row as u16),
                line,
                byte_start,
                selection.clone(),
                (horizontal_offset, content_width as usize),
                render_style,
                theme.styles.selection_range.to_render_style(),
            );
        }

        if is_focused && cursor_line >= scroll_offset && cursor_line < scroll_offset + visible_rows
        {
            let y = content_y + (cursor_line - scroll_offset) as u16;
            let g = self.value[cursor_position..]
                .graphemes(true)
                .next()
                .unwrap_or(" ");
            let width = metrics.grapheme_width(g, cursor_col);
            let g = if width == 0 || cursor_col - horizontal_offset + width > content_width as usize
            {
                " "
            } else {
                g
            };
            let _ = chunk.set_forced(
                content_x + (cursor_col - horizontal_offset) as u16,
                y,
                crate::render::style::Stylized::grapheme(g, cursor_style),
            );
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::tui::event::is_key_release(event) {
            return;
        }
        if ctx.phase() != EventPhase::Target {
            return;
        }
        if self.disabled || matches!(event, Event::FocusLost) {
            ctx.state_mut::<TextAreaState>().dragging = false;
            return;
        }
        let pointer_state = ctx.state_mut::<TextAreaState>();
        if matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left))
        {
            pointer_state.dragging = false;
        }
        let mut pointer = EditorPointer {
            anchor: pointer_state.dragging.then_some(
                pointer_state
                    .selection_anchor
                    .unwrap_or(pointer_state.cursor_position),
            ),
            scrolled: pointer_state.viewport_scrolled,
        };
        if matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Up(MouseButton::Left))
        {
            if std::mem::take(&mut ctx.state_mut::<TextAreaState>().dragging) {
                ctx.set_handled();
            }
            return;
        }
        let metrics = ctx.text_metrics();
        let local = ctx.relative_mouse_position(event);
        let bounds = ctx.bounds();
        let inset = i32::from(self.variant == TextAreaVariant::Default);
        let rows = bounds
            .map(|a| a.height())
            .unwrap_or(self.height)
            .saturating_sub(inset as u16 * 2) as usize;
        let columns = bounds
            .map(|a| a.width())
            .unwrap_or(20)
            .saturating_sub(inset as u16 * 2) as usize;
        if rows == 0 || columns == 0 {
            return;
        }
        let state = ctx.state_mut::<TextAreaState>();
        let mut editor = EditBuffer::new(
            state.value.clone().unwrap_or_else(|| self.value.clone()),
            state.cursor_position,
            state.selection_anchor,
        );
        if let Some(delta) = pointer::wheel_delta(event) {
            let max_offset = editor.value.split('\n').count().saturating_sub(rows);
            let old = state.scroll_offset.min(max_offset);
            let next = old.saturating_add_signed(delta).min(max_offset);
            state.scroll_offset = next;
            if next != old {
                pointer.scrolled = true;
                state.dragging = pointer.anchor.is_some();
                state.viewport_scrolled = pointer.scrolled;
                ctx.set_handled();
            }
            return;
        }
        let result = if let Event::Mouse(mouse) = event {
            let Some((x, y)) = local else {
                return;
            };
            let dragging = mouse.kind == MouseEventKind::Drag(MouseButton::Left);
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left)
                    if x >= inset
                        && y >= inset
                        && x < columns as i32 + inset
                        && y < rows as i32 + inset =>
                {
                    pointer.anchor = None;
                }
                MouseEventKind::Drag(MouseButton::Left) if pointer.anchor.is_some() => {}
                _ => return,
            }
            let (line, col) = metrics.byte_to_line_column(&editor.value, editor.cursor);
            let mut scroll = if pointer.scrolled {
                state
                    .scroll_offset
                    .min(editor.value.split('\n').count().saturating_sub(rows))
            } else {
                ensure_item_visible(state.scroll_offset, line, rows)
            };
            let mut horizontal = cursor_scroll(col, state.horizontal_offset, columns);
            if dragging {
                if y < inset {
                    scroll = scroll.saturating_sub(1);
                }
                if y >= inset + rows as i32 {
                    scroll = scroll
                        .saturating_add(1)
                        .min(editor.value.split('\n').count().saturating_sub(rows));
                }
                if x < inset {
                    horizontal = horizontal.saturating_sub(1);
                }
                if x >= inset + columns as i32 {
                    horizontal = horizontal.saturating_add(1);
                }
            }
            let old_cursor = editor.cursor;
            editor.cursor = metrics.line_column_to_byte(
                &editor.value,
                scroll + (y - inset).clamp(0, rows as i32 - 1) as usize,
                horizontal + (x - inset).clamp(0, columns as i32 - 1) as usize,
            );
            if dragging {
                editor.anchor = pointer.anchor.map(|a| floor_boundary(&editor.value, a));
            } else if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                let anchor = editor.anchor.unwrap_or(old_cursor);
                editor.anchor = Some(anchor);
                pointer.anchor = Some(anchor);
            } else {
                pointer.anchor = Some(editor.cursor);
                editor.anchor = None;
            }
            pointer.scrolled = false;
            state.scroll_offset = scroll;
            state.horizontal_offset = horizontal;
            crate::tui::text::EditResult {
                handled: true,
                ..Default::default()
            }
        } else {
            pointer.anchor = None;
            pointer.scrolled = false;
            editor.handle(event, metrics, true)
        };
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
        let (line, col) = metrics.byte_to_line_column(&editor.value, editor.cursor);
        state.scroll_offset = ensure_item_visible(state.scroll_offset, line, rows);
        state.horizontal_offset = cursor_scroll(col, state.horizontal_offset, columns);
        state.value = Some(editor.value);
        state.dragging = pointer.anchor.is_some();
        state.viewport_scrolled = pointer.scrolled;
        ctx.set_handled();
        if let Some(message) = message {
            ctx.emit(message);
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

    fn layout_style_with(&self, ctx: &crate::tui::widget::MeasureCtx) -> LayoutStyle {
        let mut style = self.layout_style();
        let width = self.preferred_width_with(ctx.theme().text_metrics);
        style.min_width = width;
        style.preferred_width = Some(width);
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

pub fn textarea<M>() -> TextArea<M> {
    TextArea::new()
}

impl<M> TextArea<M> {
    fn preferred_width(&self) -> u16 {
        self.preferred_width_with(TextMetrics::default())
    }

    fn preferred_width_with(&self, metrics: TextMetrics) -> u16 {
        let value_width = self
            .value
            .split('\n')
            .map(|line| metrics.width(line).min(u16::MAX as usize) as u16)
            .max()
            .unwrap_or(0);
        let placeholder_width = self
            .placeholder
            .as_deref()
            .map(|placeholder| metrics.width(placeholder).min(u16::MAX as usize) as u16)
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
