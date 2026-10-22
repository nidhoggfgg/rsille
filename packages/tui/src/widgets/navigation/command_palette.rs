//! Searchable command palette with fuzzy matching.

use render::text::TextWidth;

use crate::event::{Event, KeyCode};
use crate::focus::FocusConfig;
use crate::layout::border_renderer;
use crate::layout::{ensure_item_visible, Constraints, LayoutStyle};
use crate::style::{BorderStyle, Style};
use crate::widget::{EventCtx, EventPhase, RenderCtx, Widget};

use crate::widgets::collections::selection::SelectionState;
use crate::widgets::controls::select::{match_search_score, SelectSearchMode};

/// A searchable command palette entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandItem {
    pub id: String,
    pub label: String,
    pub keywords: Vec<String>,
    pub disabled: bool,
}

impl CommandItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            keywords: Vec::new(),
            disabled: false,
        }
    }

    pub fn keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    pub fn keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords.extend(keywords.into_iter().map(Into::into));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Persistent command palette state stored in the widget store.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandPaletteState {
    pub query: String,
    pub selection: SelectionState,
    pub scroll_offset: usize,
}

/// A fuzzy-searchable command palette widget.
pub struct CommandPalette<M = ()> {
    items: Vec<CommandItem>,
    height: u16,
    border: BorderStyle,
    title: Option<String>,
    prompt: String,
    placeholder: String,
    empty_message: String,
    disabled: bool,
    search_mode: SelectSearchMode,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(String) -> M>>,
    on_submit: Option<Box<dyn Fn(String) -> M>>,
    on_close: Option<Box<dyn Fn() -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for CommandPalette<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandPalette")
            .field("items", &self.items)
            .field("height", &self.height)
            .field("border", &self.border)
            .field("title", &self.title)
            .field("prompt", &self.prompt)
            .field("placeholder", &self.placeholder)
            .field("empty_message", &self.empty_message)
            .field("disabled", &self.disabled)
            .field("search_mode", &self.search_mode)
            .field("on_change", &self.on_change.is_some())
            .field("on_submit", &self.on_submit.is_some())
            .field("on_close", &self.on_close.is_some())
            .finish()
    }
}

impl<M> CommandPalette<M> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            height: 10,
            border: BorderStyle::Double,
            title: Some("Command Palette".to_owned()),
            prompt: ">".to_owned(),
            placeholder: "Type to search commands".to_owned(),
            empty_message: "No matching commands".to_owned(),
            disabled: false,
            search_mode: SelectSearchMode::Fuzzy,
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            on_submit: None,
            on_close: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn item(mut self, item: CommandItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = CommandItem>,
    {
        self.items.extend(items);
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height.max(5);
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = border;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn title_opt(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = message.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn search_mode(mut self, mode: SelectSearchMode) -> Self {
        self.search_mode = mode;
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

    pub fn on_close<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + 'static,
    {
        self.on_close = Some(Box::new(handler));
        self
    }

    fn visible_rows(&self) -> usize {
        let chrome = 4 + usize::from(self.title.is_some());
        self.height.saturating_sub(chrome as u16) as usize
    }

    fn filtered_indices(&self, query: &str) -> Vec<usize> {
        let trimmed = query.trim();
        let mut scored = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.disabled)
            .filter_map(|(index, item)| {
                if trimmed.is_empty() {
                    return Some((index, 0usize));
                }

                let best = std::iter::once(item.label.as_str())
                    .chain(item.keywords.iter().map(String::as_str))
                    .filter_map(|candidate| {
                        match_search_score(candidate, trimmed, self.search_mode)
                    })
                    .min()?;
                Some((index, best))
            })
            .collect::<Vec<_>>();

        scored.sort_by_key(|(index, score)| (*score, *index));
        scored.into_iter().map(|(index, _)| index).collect()
    }

    fn active_filtered_position(
        &self,
        state: &CommandPaletteState,
        filtered: &[usize],
    ) -> Option<usize> {
        state
            .selection
            .cursor()
            .and_then(|id| {
                filtered
                    .iter()
                    .position(|index| self.items[*index].id == id && !self.items[*index].disabled)
            })
            .or_else(|| {
                filtered
                    .iter()
                    .position(|index| !self.items[*index].disabled)
            })
    }

    fn sync_cursor(&self, state: &mut CommandPaletteState, filtered: &[usize]) -> Option<usize> {
        let Some(position) = self.active_filtered_position(state, filtered) else {
            state.selection.set_cursor(None);
            return None;
        };

        let item_id = self.items[filtered[position]].id.clone();
        state.selection.set_cursor(Some(item_id));
        Some(position)
    }

    fn truncate_to_width(
        text: &str,
        max_width: usize,
        metrics: render::text::TextMetrics,
    ) -> String {
        metrics.clip(text, 0, max_width)
    }
}

impl<M> Default for CommandPalette<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for CommandPalette<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let theme = ctx.theme();
        let is_focused = ctx.is_focused();
        let base_style = if self.disabled {
            theme.styles.interactive_disabled
        } else {
            theme.styles.surface_popup
        };
        let field_style = self
            .custom_style
            .as_ref()
            .map(|style| style.merge(base_style))
            .unwrap_or(base_style)
            .to_render_style();
        let active_style = if is_focused {
            self.custom_focus_style
                .unwrap_or(theme.styles.selected_focused)
        } else {
            theme.styles.selected
        }
        .to_render_style();
        let header_style = theme.styles.surface_header.to_render_style();
        let muted_style = theme.styles.text_muted.to_render_style();
        let border_style = if is_focused {
            theme.styles.border_focused.to_render_style()
        } else {
            theme.styles.border.to_render_style()
        };

        let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', field_style);
        if area.width() < 2 || area.height() < 2 {
            return;
        }
        border_renderer::render_border(chunk, self.border, border_style);

        let content_x = 1u16;
        let content_y = 1u16;
        let content_width = area.width() - 2;
        let content_height = area.height() - 2;
        if content_width == 0 || content_height == 0 {
            return;
        }

        let state = ctx.state_or_default::<CommandPaletteState>();
        let filtered = self.filtered_indices(&state.query);

        let mut cursor_y = content_y;
        if let Some(title) = self.title.as_deref() {
            let line = Self::truncate_to_width(title, content_width as usize, chunk.text_metrics());
            let _ = chunk.fill(content_x, cursor_y, content_width, 1, ' ', header_style);
            let _ = chunk.set_string(content_x, cursor_y, &line, header_style);
            cursor_y = cursor_y.saturating_add(1);
        }

        let prompt = format!("{} ", self.prompt);
        let query_text = if state.query.is_empty() {
            self.placeholder.as_str()
        } else {
            state.query.as_str()
        };
        let display_style = if state.query.is_empty() {
            muted_style
        } else {
            field_style
        };
        let available =
            content_width.saturating_sub(chunk.text_metrics().width(&prompt) as u16) as usize;
        let query = Self::truncate_to_width(query_text, available, chunk.text_metrics());
        let _ = chunk.fill(content_x, cursor_y, content_width, 1, ' ', field_style);
        let _ = chunk.set_string(content_x, cursor_y, &prompt, field_style);
        let _ = chunk.set_string(
            content_x + chunk.text_metrics().width(&prompt) as u16,
            cursor_y,
            &query,
            display_style,
        );
        cursor_y = cursor_y.saturating_add(1);

        let _ = chunk.fill(content_x, cursor_y, content_width, 1, '-', muted_style);
        cursor_y = cursor_y.saturating_add(1);

        if filtered.is_empty() {
            let message = Self::truncate_to_width(
                &self.empty_message,
                content_width as usize,
                chunk.text_metrics(),
            );
            let _ = chunk.set_string(content_x, cursor_y, &message, muted_style);
            return;
        }

        let active_position = self.active_filtered_position(state, &filtered);
        let visible_rows = content_height
            .saturating_sub(cursor_y.saturating_sub(content_y))
            .max(1) as usize;
        let mut scroll_offset = state.scroll_offset.min(filtered.len().saturating_sub(1));
        if let Some(position) = active_position {
            scroll_offset = ensure_item_visible(scroll_offset, position, visible_rows);
        }

        for row in 0..visible_rows {
            let filtered_position = scroll_offset + row;
            if filtered_position >= filtered.len() {
                break;
            }

            let item = &self.items[filtered[filtered_position]];
            let is_active = active_position == Some(filtered_position);
            let style = if is_active { active_style } else { field_style };
            let y = cursor_y + row as u16;
            let _ = chunk.fill(content_x, y, content_width, 1, ' ', style);

            let prefix = if is_active { "> " } else { "  " };
            let available =
                content_width.saturating_sub(chunk.text_metrics().width(&prefix) as u16) as usize;
            let text = Self::truncate_to_width(&item.label, available, chunk.text_metrics());
            let line = format!("{prefix}{text}");
            let _ = chunk.set_string(content_x, y, &line, style);
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target || self.disabled {
            return;
        }

        let Event::Key(key_event) = event else {
            return;
        };

        let mut emit_change = None;
        let mut emit_submit = None;
        let mut emit_close = None;
        let mut handled = true;

        {
            let state = ctx.state_mut::<CommandPaletteState>();
            let mut filtered = self.filtered_indices(&state.query);
            let Some(mut active_position) = self.sync_cursor(state, &filtered) else {
                match key_event.code {
                    KeyCode::Char(c) => {
                        state.query.push(c);
                        filtered = self.filtered_indices(&state.query);
                        let _ = self.sync_cursor(state, &filtered);
                    }
                    KeyCode::Backspace => {
                        state.query.pop();
                        filtered = self.filtered_indices(&state.query);
                        let _ = self.sync_cursor(state, &filtered);
                    }
                    KeyCode::Esc => {
                        if let Some(ref handler) = self.on_close {
                            emit_close = Some(handler());
                        }
                    }
                    _ => handled = false,
                }

                if handled {
                    ctx.set_handled();
                }
                if let Some(message) = emit_close {
                    ctx.emit(message);
                }
                return;
            };

            match key_event.code {
                KeyCode::Up => {
                    active_position = active_position.saturating_sub(1);
                }
                KeyCode::Down => {
                    if active_position + 1 < filtered.len() {
                        active_position += 1;
                    }
                }
                KeyCode::Home => active_position = 0,
                KeyCode::End => active_position = filtered.len().saturating_sub(1),
                KeyCode::PageUp => {
                    active_position = active_position.saturating_sub(self.visible_rows().max(1));
                }
                KeyCode::PageDown => {
                    active_position = (active_position + self.visible_rows().max(1))
                        .min(filtered.len().saturating_sub(1));
                }
                KeyCode::Backspace => {
                    state.query.pop();
                    filtered = self.filtered_indices(&state.query);
                    active_position = 0;
                }
                KeyCode::Char(c) => {
                    state.query.push(c);
                    filtered = self.filtered_indices(&state.query);
                    active_position = 0;
                }
                KeyCode::Enter => {
                    let item_id = self.items[filtered[active_position]].id.clone();
                    if let Some(ref handler) = self.on_submit {
                        emit_submit = Some(handler(item_id));
                    }
                }
                KeyCode::Esc => {
                    if let Some(ref handler) = self.on_close {
                        emit_close = Some(handler());
                    }
                }
                _ => handled = false,
            }

            if !filtered.is_empty() {
                active_position = active_position.min(filtered.len().saturating_sub(1));
                let item_id = self.items[filtered[active_position]].id.clone();
                state.selection.set_cursor(Some(item_id.clone()));
                state.scroll_offset = ensure_item_visible(
                    state.scroll_offset,
                    active_position,
                    self.visible_rows().max(1),
                );
                if let Some(ref handler) = self.on_change {
                    emit_change = Some(handler(item_id));
                }
            } else {
                state.selection.set_cursor(None);
                state.scroll_offset = 0;
            }
        }

        if handled {
            ctx.set_handled();
        }
        if let Some(message) = emit_change {
            ctx.emit(message);
        }
        if let Some(message) = emit_submit {
            ctx.emit(message);
        }
        if let Some(message) = emit_close {
            ctx.emit(message);
        }
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: self.preferred_width(),
            max_width: None,
            min_height: self.height,
            max_height: Some(self.height),
            flex: None,
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
            FocusConfig::Composite
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a new command palette widget.
pub fn command_palette<M>() -> CommandPalette<M> {
    CommandPalette::new()
}

impl<M> CommandPalette<M> {
    fn preferred_width(&self) -> u16 {
        let widest = self
            .items
            .iter()
            .map(|item| item.label.width() as u16)
            .chain(self.title.iter().map(|title| title.width() as u16))
            .max()
            .unwrap_or(20) as u16;
        widest
            .max(self.placeholder.width() as u16)
            .saturating_add(self.prompt.width() as u16)
            .saturating_add(7)
    }
}
