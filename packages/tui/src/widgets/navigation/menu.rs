//! Menu widget with roving selection.

use render::text::TextWidth;

use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::focus::FocusConfig;
use crate::layout::border_renderer;
use crate::layout::{ensure_item_visible, Constraints};
use crate::style::{BorderStyle, Style};
use crate::widget::{EventCtx, EventPhase, RenderCtx, Widget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub disabled: bool,
}

impl MenuItem {
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

impl From<&str> for MenuItem {
    fn from(value: &str) -> Self {
        Self::new(value, value)
    }
}

impl From<String> for MenuItem {
    fn from(value: String) -> Self {
        Self::new(value.clone(), value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MenuState {
    pub active_item: Option<String>,
    pub scroll_offset: usize,
}

pub struct Menu<M = ()> {
    items: Vec<MenuItem>,
    height: u16,
    border: Option<BorderStyle>,
    disabled: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_select: Option<Box<dyn Fn(String) -> M>>,
    on_close: Option<Box<dyn Fn() -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Menu<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Menu")
            .field("items", &self.items)
            .field("height", &self.height)
            .field("border", &self.border)
            .field("disabled", &self.disabled)
            .field("on_select", &self.on_select.is_some())
            .field("on_close", &self.on_close.is_some())
            .finish()
    }
}

impl<M> Menu<M> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            height: 6,
            border: Some(BorderStyle::Single),
            disabled: false,
            custom_style: None,
            custom_focus_style: None,
            on_select: None,
            on_close: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn item(mut self, item: impl Into<MenuItem>) -> Self {
        self.items.push(item.into());
        self
    }

    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<MenuItem>,
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

    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    pub fn on_select<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_select = Some(Box::new(handler));
        self
    }

    pub fn on_close<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + 'static,
    {
        self.on_close = Some(Box::new(handler));
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

    fn active_index_from_state(&self, state: &MenuState) -> Option<usize> {
        state
            .active_item
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

impl<M> Default for Menu<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for Menu<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let theme = ctx.theme();
        let row_style = self
            .custom_style
            .map(|style| style.merge(theme.styles.surface_popup))
            .unwrap_or(theme.styles.surface_popup);
        let active_style = if ctx.is_focused() {
            self.custom_focus_style
                .map(|style| style.merge(theme.styles.menu_item_active_focused))
                .unwrap_or(theme.styles.menu_item_active_focused)
        } else {
            self.custom_focus_style
                .map(|style| style.merge(theme.styles.menu_item_active))
                .unwrap_or(theme.styles.menu_item_active)
        };
        let row_render_style = row_style.to_render_style();
        let active_render_style = active_style.to_render_style();
        let disabled_style = theme
            .styles
            .interactive_disabled
            .merge(row_style)
            .to_render_style();
        let border_style = if ctx.is_focused() {
            theme.styles.border_focused.to_render_style()
        } else {
            theme.styles.border.to_render_style()
        };

        let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', row_render_style);
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

        let state = ctx.state_or_default::<MenuState>();
        let active_index = self.active_index_from_state(state);
        let visible_rows = content_height as usize;
        let mut scroll_offset = state.scroll_offset.min(self.items.len().saturating_sub(1));
        if let Some(index) = active_index {
            scroll_offset = ensure_item_visible(scroll_offset, index, visible_rows.max(1));
        }

        for row in 0..visible_rows {
            let item_index = scroll_offset + row;
            if item_index >= self.items.len() {
                break;
            }

            let item = &self.items[item_index];
            let is_active = active_index == Some(item_index);
            let style = if self.disabled || item.disabled {
                disabled_style
            } else if is_active {
                active_render_style
            } else {
                row_render_style
            };
            let y = content_y + row as u16;
            let _ = chunk.fill(content_x, y, content_width, 1, ' ', style);
            let prefix = if is_active { "> " } else { "  " };
            let text = truncate_to_width(
                &format!("{prefix}{}", item.label),
                content_width as usize,
                chunk.text_metrics(),
            );
            let _ = chunk.set_string(content_x, y, &text, style);
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target || self.disabled {
            return;
        }

        if matches!(event, Event::Key(key_event) if key_event.code == KeyCode::Esc) {
            ctx.set_handled();
            if let Some(handler) = self.on_close.as_ref() {
                ctx.emit(handler());
            }
            return;
        }

        if !self.has_enabled_items() {
            return;
        }

        let clicked_row = match event {
            Event::Mouse(mouse_event)
                if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left)) =>
            {
                ctx.local_mouse_position(event).and_then(|(_, row)| {
                    row.checked_sub(u16::from(self.border.is_some()))
                        .map(|row| row as usize)
                })
            }
            _ => None,
        };
        let visible_rows = (self
            .height
            .saturating_sub(if self.border.is_some() { 2 } else { 0 }))
        .max(1) as usize;
        let mut select_id = None;
        {
            let state = ctx.state_mut::<MenuState>();
            let Some(mut active_index) = self.active_index_from_state(state) else {
                return;
            };

            match event {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Up => {
                        if let Some(index) = self.prev_enabled_index(active_index) {
                            active_index = index;
                        }
                    }
                    KeyCode::Down => {
                        if let Some(index) = self.next_enabled_index(active_index) {
                            active_index = index;
                        }
                    }
                    KeyCode::Home => {
                        if let Some(index) = self.first_enabled_index() {
                            active_index = index;
                        }
                    }
                    KeyCode::End => {
                        if let Some(index) = self.last_enabled_index() {
                            active_index = index;
                        }
                    }
                    KeyCode::PageUp => {
                        for _ in 0..visible_rows {
                            if let Some(index) = self.prev_enabled_index(active_index) {
                                active_index = index;
                            } else {
                                break;
                            }
                        }
                    }
                    KeyCode::PageDown => {
                        for _ in 0..visible_rows {
                            if let Some(index) = self.next_enabled_index(active_index) {
                                active_index = index;
                            } else {
                                break;
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        select_id = Some(self.items[active_index].id.clone());
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
                    select_id = Some(self.items[active_index].id.clone());
                }
                _ => return,
            }

            state.active_item = Some(self.items[active_index].id.clone());
            state.scroll_offset =
                ensure_item_visible(state.scroll_offset, active_index, visible_rows);
        }

        ctx.set_handled();
        if let (Some(id), Some(handler)) = (select_id, self.on_select.as_ref()) {
            ctx.emit(handler(id));
        }
    }

    fn constraints(&self) -> Constraints {
        let widest = self
            .items
            .iter()
            .map(|item| item.label.width() as u16)
            .max()
            .unwrap_or(10);
        Constraints {
            min_width: widest + 4,
            max_width: None,
            min_height: self.height,
            max_height: Some(self.height),
            flex: None,
        }
    }

    fn focus_config(&self) -> FocusConfig {
        if self.disabled || (!self.has_enabled_items() && self.on_close.is_none()) {
            FocusConfig::None
        } else {
            FocusConfig::Composite
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

fn truncate_to_width(text: &str, max_width: usize, metrics: render::text::TextMetrics) -> String {
    metrics.clip(text, 0, max_width)
}

pub fn menu<M>() -> Menu<M> {
    Menu::new()
}
