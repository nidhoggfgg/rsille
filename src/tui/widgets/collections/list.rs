//! List widget with roving internal selection.

use crate::tui::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::border_renderer;
use crate::tui::layout::{ensure_item_visible, Constraints, LayoutStyle};
use crate::tui::style::{BorderStyle, Style};
use crate::tui::widget::{EventCtx, EventPhase, RenderCtx, Widget};
use crate::tui::widgets::pointer;

use super::selection::{SelectionMode, SelectionState};

/// A single list item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub id: String,
    pub label: String,
    pub disabled: bool,
}

impl ListItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl From<&str> for ListItem {
    fn from(value: &str) -> Self {
        Self::new(value, value)
    }
}

impl From<String> for ListItem {
    fn from(value: String) -> Self {
        Self::new(value.clone(), value)
    }
}

/// Persistent list state stored in the widget store.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    pub active_item: Option<String>,
    pub selection: SelectionState,
    pub scroll_offset: usize,
}

/// Focusable list widget with internal arrow-key navigation.
pub struct List<M = ()> {
    items: Vec<ListItem>,
    height: u16,
    border: Option<BorderStyle>,
    disabled: bool,
    selection_mode: SelectionMode,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(String) -> M>>,
    on_submit: Option<Box<dyn Fn(String) -> M>>,
    on_selection_change: Option<Box<dyn Fn(Vec<String>) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for List<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("List")
            .field("items", &self.items)
            .field("height", &self.height)
            .field("border", &self.border)
            .field("disabled", &self.disabled)
            .field("selection_mode", &self.selection_mode)
            .field("on_change", &self.on_change.is_some())
            .field("on_submit", &self.on_submit.is_some())
            .field("on_selection_change", &self.on_selection_change.is_some())
            .finish()
    }
}

impl<M> List<M> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            height: 6,
            border: Some(BorderStyle::Single),
            disabled: false,
            selection_mode: SelectionMode::Single,
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            on_submit: None,
            on_selection_change: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn item(mut self, item: impl Into<ListItem>) -> Self {
        self.items.push(item.into());
        self
    }

    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<ListItem>,
    {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height.max(1);
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    pub fn borderless(mut self) -> Self {
        self.border = None;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    pub fn multi_select(mut self, multi_select: bool) -> Self {
        self.selection_mode = if multi_select {
            SelectionMode::Multiple
        } else {
            SelectionMode::Single
        };
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

    pub fn on_selection_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(Vec<String>) -> M + 'static,
    {
        self.on_selection_change = Some(Box::new(handler));
        self
    }

    fn has_enabled_items(&self) -> bool {
        self.items.iter().any(|item| !item.disabled)
    }

    fn first_enabled_index(&self) -> Option<usize> {
        self.items.iter().position(|item| !item.disabled)
    }

    fn last_enabled_index(&self) -> Option<usize> {
        self.items.iter().rposition(|item| !item.disabled)
    }

    fn index_for_id(&self, id: &str) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.id == id && !item.disabled)
    }

    fn active_index_from_state(&self, state: &ListState) -> Option<usize> {
        active_item_id(state)
            .as_deref()
            .and_then(|id| self.index_for_id(id))
            .or_else(|| self.first_enabled_index())
    }

    fn next_enabled_index(&self, current: usize) -> Option<usize> {
        self.items
            .iter()
            .enumerate()
            .skip(current.saturating_add(1))
            .find(|(_, item)| !item.disabled)
            .map(|(index, _)| index)
    }

    fn prev_enabled_index(&self, current: usize) -> Option<usize> {
        self.items
            .iter()
            .enumerate()
            .take(current)
            .rev()
            .find(|(_, item)| !item.disabled)
            .map(|(index, _)| index)
    }
}

impl<M> Default for List<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for List<M> {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let is_focused = ctx.is_focused();
        let theme = ctx.theme();
        let metrics = theme.text_metrics;
        let base_style = if self.disabled {
            theme.styles.interactive_disabled
        } else {
            theme.styles.surface
        };
        let active_style = if is_focused {
            self.custom_focus_style
                .unwrap_or(theme.styles.list_active_focused)
        } else {
            theme.styles.list_active
        };
        let selected_style = theme.styles.selected.to_render_style();
        let row_style = self
            .custom_style
            .as_ref()
            .map(|style| style.merge(base_style))
            .unwrap_or(base_style)
            .to_render_style();
        let active_row_style = active_style.to_render_style();
        let disabled_row_style = theme.styles.interactive_disabled.to_render_style();
        let border_style = if is_focused {
            theme.styles.border_focused.to_render_style()
        } else {
            theme.styles.border.to_render_style()
        };

        let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', row_style);

        let (content_x, content_y, content_width, content_height) =
            if let Some(border) = self.border {
                if area.width() < 2 || area.height() < 2 {
                    return;
                }
                border_renderer::render_border(chunk, border, border_style);
                (1u16, 1u16, area.width() - 2, area.height() - 2)
            } else {
                (0u16, 0u16, area.width(), area.height())
            };

        if content_width == 0 || content_height == 0 {
            return;
        }

        let state = ctx.state_or_default::<ListState>();
        let Some(active_index) = self.active_index_from_state(state) else {
            return;
        };

        let visible_rows = content_height as usize;
        let mut scroll_offset = state.scroll_offset.min(self.items.len().saturating_sub(1));
        scroll_offset = ensure_item_visible(scroll_offset, active_index, visible_rows);

        for row in 0..visible_rows {
            let item_index = scroll_offset + row;
            if item_index >= self.items.len() {
                break;
            }

            let item = &self.items[item_index];
            let is_active = item_index == active_index;
            let is_selected = state.selection.is_selected(&item.id);
            let style = if item.disabled {
                disabled_row_style
            } else if is_active {
                active_row_style
            } else if is_selected {
                selected_style
            } else {
                row_style
            };

            let y = content_y + row as u16;
            let _ = chunk.fill(content_x, y, content_width, 1, ' ', style);

            let prefix = if is_active && is_selected {
                ">* "
            } else if is_active {
                ">  "
            } else if is_selected {
                " * "
            } else {
                "   "
            };
            let available_width =
                content_width.saturating_sub(metrics.width(prefix) as u16) as usize;
            let text = metrics.clip(&item.label, 0, available_width);
            let line = format!("{prefix}{text}");
            let _ = chunk.set_string(content_x, y, &line, style);
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::tui::event::is_key_release(event) {
            return;
        }
        if ctx.phase() != EventPhase::Target || self.disabled || !self.has_enabled_items() {
            return;
        }

        if let Some(delta) = pointer::wheel_delta(event) {
            let visible = pointer::visible_rows(ctx, self.border.is_some(), self.height, 0);
            let state = ctx.state_mut::<ListState>();
            let Some(active) = self.active_index_from_state(state) else {
                return;
            };
            let Some((offset, index)) = pointer::scroll_cursor(
                state.scroll_offset,
                active,
                self.items.len(),
                visible,
                delta,
                |i| !self.items[i].disabled,
            ) else {
                return;
            };
            state.scroll_offset = offset;
            state.active_item = Some(self.items[index].id.clone());
            state.selection.set_cursor(state.active_item.clone());
            let message = self
                .on_change
                .as_ref()
                .map(|handler| handler(self.items[index].id.clone()));
            if let Some(message) = message {
                ctx.emit(message);
            }
            ctx.set_handled();
            return;
        }
        let clicked_row =
            pointer::content_position(ctx, event, self.border.is_some(), 0).map(|(_, row)| row);
        let visible_rows = pointer::visible_rows(ctx, self.border.is_some(), self.height, 0);
        if visible_rows == 0 {
            return;
        }
        let mut emit_submit = None;
        let mut emit_selection = None;
        let mut moved = false;

        let next_active_id = {
            let state = ctx.state_mut::<ListState>();
            let Some(mut active_index) = self.active_index_from_state(state) else {
                return;
            };

            match event {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Up => {
                        if let Some(index) = self.prev_enabled_index(active_index) {
                            active_index = index;
                            moved = true;
                        }
                    }
                    KeyCode::Down => {
                        if let Some(index) = self.next_enabled_index(active_index) {
                            active_index = index;
                            moved = true;
                        }
                    }
                    KeyCode::Home => {
                        if let Some(index) = self.first_enabled_index() {
                            active_index = index;
                            moved = true;
                        }
                    }
                    KeyCode::End => {
                        if let Some(index) = self.last_enabled_index() {
                            active_index = index;
                            moved = true;
                        }
                    }
                    KeyCode::PageUp => {
                        for _ in 0..visible_rows {
                            if let Some(index) = self.prev_enabled_index(active_index) {
                                active_index = index;
                                moved = true;
                            } else {
                                break;
                            }
                        }
                    }
                    KeyCode::PageDown => {
                        for _ in 0..visible_rows {
                            if let Some(index) = self.next_enabled_index(active_index) {
                                active_index = index;
                                moved = true;
                            } else {
                                break;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        let active_id = self.items[active_index].id.clone();
                        if self.selection_mode == SelectionMode::Single {
                            state.selection.replace_selection(active_id.clone());
                        }
                        if let Some(ref handler) = self.on_submit {
                            emit_submit = Some(handler(active_id));
                        }
                        ctx.set_handled();
                        if let Some(message) = emit_submit {
                            ctx.emit(message);
                        }
                        return;
                    }
                    KeyCode::Char(' ') if self.selection_mode.is_multiple() => {
                        let active_id = self.items[active_index].id.clone();
                        state.selection.toggle(&active_id);
                        let emit_selection = self
                            .on_selection_change
                            .as_ref()
                            .map(|handler| handler(state.selection.selected.clone()));
                        ctx.set_handled();
                        if let Some(message) = emit_selection {
                            ctx.emit(message);
                        }
                        return;
                    }
                    _ => return,
                },
                Event::Mouse(mouse_event)
                    if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left)) =>
                {
                    let Some(row) = clicked_row else {
                        return;
                    };
                    if row >= visible_rows {
                        return;
                    }
                    let mut visible_offset =
                        state.scroll_offset.min(self.items.len().saturating_sub(1));
                    visible_offset =
                        ensure_item_visible(visible_offset, active_index, visible_rows);
                    let index = visible_offset + row;
                    if index >= self.items.len() || self.items[index].disabled {
                        return;
                    }
                    active_index = index;
                    moved = true;

                    let active_id = self.items[active_index].id.clone();
                    if self.selection_mode == SelectionMode::Single {
                        state.selection.replace_selection(active_id);
                    } else {
                        state.selection.toggle(&active_id);
                        emit_selection = self
                            .on_selection_change
                            .as_ref()
                            .map(|handler| handler(state.selection.selected.clone()));
                    }
                }
                _ => return,
            }

            state.active_item = Some(self.items[active_index].id.clone());
            state.selection.set_cursor(state.active_item.clone());
            state.scroll_offset =
                ensure_item_visible(state.scroll_offset, active_index, visible_rows);
            state.active_item.clone()
        };

        if moved {
            ctx.set_handled();
            if let (Some(active_id), Some(handler)) = (next_active_id, self.on_change.as_ref()) {
                ctx.emit(handler(active_id));
            }
            if let Some(message) = emit_selection {
                ctx.emit(message);
            }
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
        if self.disabled || !self.has_enabled_items() {
            FocusConfig::None
        } else {
            FocusConfig::Composite
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a new list widget.
pub fn list<M>() -> List<M> {
    List::new()
}

fn active_item_id(state: &ListState) -> Option<String> {
    state
        .selection
        .cursor
        .clone()
        .or_else(|| state.active_item.clone())
}

impl<M> List<M> {
    fn preferred_width(&self) -> u16 {
        self.preferred_width_with(crate::render::text::TextMetrics::default())
    }

    fn preferred_width_with(&self, metrics: crate::render::text::TextMetrics) -> u16 {
        self.items
            .iter()
            .map(|item| metrics.width(&item.label).min(u16::MAX as usize) as u16)
            .max()
            .unwrap_or(10)
            .saturating_add(4)
            .saturating_add(if self.border.is_some() { 2 } else { 0 })
    }
}
