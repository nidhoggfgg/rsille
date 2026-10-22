//! Select widget with expandable option list.

use crate::render::text::TextWidth;

use crate::tui::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::border_renderer;
use crate::tui::layout::{ensure_item_visible, Constraints, LayoutStyle};
use crate::tui::style::{BorderStyle, Style};
use crate::tui::widget::{EventCtx, EventPhase, RenderCtx, Widget};

use crate::tui::widgets::collections::selection::SelectionState;

/// A single select option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl From<&str> for SelectOption {
    fn from(value: &str) -> Self {
        Self::new(value, value)
    }
}

impl From<String> for SelectOption {
    fn from(value: String) -> Self {
        Self::new(value.clone(), value)
    }
}

/// Search strategy used by searchable selects and command palettes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectSearchMode {
    #[default]
    Contains,
    Fuzzy,
}

/// Persistent select state stored in the widget store.
#[derive(Debug, Clone, Default)]
pub struct SelectState {
    pub is_open: bool,
    pub active_option: Option<String>,
    pub selected_option: Option<String>,
    pub search_query: String,
    pub selection: SelectionState,
    pub scroll_offset: usize,
}

/// Focusable select widget with an inline option menu.
pub struct Select<M = ()> {
    options: Vec<SelectOption>,
    height: u16,
    border: Option<BorderStyle>,
    placeholder: String,
    disabled: bool,
    searchable: bool,
    search_mode: SelectSearchMode,
    empty_search_message: String,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(String) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Select<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("options", &self.options)
            .field("height", &self.height)
            .field("border", &self.border)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("searchable", &self.searchable)
            .field("search_mode", &self.search_mode)
            .field("on_change", &self.on_change.is_some())
            .finish()
    }
}

impl<M> Select<M> {
    pub fn new() -> Self {
        Self {
            options: Vec::new(),
            height: 7,
            border: Some(BorderStyle::Single),
            placeholder: "Choose an option".to_owned(),
            disabled: false,
            searchable: false,
            search_mode: SelectSearchMode::Contains,
            empty_search_message: "No matching options".to_owned(),
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn option(mut self, option: impl Into<SelectOption>) -> Self {
        self.options.push(option.into());
        self
    }

    pub fn options<I>(mut self, options: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<SelectOption>,
    {
        self.options.extend(options.into_iter().map(Into::into));
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height.max(4);
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

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    pub fn search_mode(mut self, mode: SelectSearchMode) -> Self {
        self.search_mode = mode;
        self
    }

    pub fn empty_search_message(mut self, message: impl Into<String>) -> Self {
        self.empty_search_message = message.into();
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

    fn has_enabled_options(&self) -> bool {
        self.options.iter().any(|option| !option.disabled)
    }

    fn option_matches_query(&self, option: &SelectOption, query: &str) -> bool {
        match_search_score(&option.label, query, self.search_mode).is_some()
    }

    fn filtered_indices(&self, state: &SelectState) -> Vec<usize> {
        let query = state.search_query.trim();

        self.options
            .iter()
            .enumerate()
            .filter(|(_, option)| !option.disabled)
            .filter(|(_, option)| query.is_empty() || self.option_matches_query(option, query))
            .map(|(index, _)| index)
            .collect()
    }

    fn selected_label<'a>(&'a self, state: &'a SelectState) -> Option<&'a str> {
        current_selected_value(state)
            .as_deref()
            .and_then(|value| self.options.iter().find(|option| option.value == value))
            .map(|option| option.label.as_str())
    }

    fn active_index_from_state(&self, state: &SelectState, filtered: &[usize]) -> Option<usize> {
        active_value(state)
            .as_deref()
            .and_then(|value| {
                filtered
                    .iter()
                    .copied()
                    .find(|index| self.options[*index].value == value)
            })
            .or_else(|| {
                current_selected_value(state).as_deref().and_then(|value| {
                    filtered
                        .iter()
                        .copied()
                        .find(|index| self.options[*index].value == value)
                })
            })
            .or_else(|| filtered.first().copied())
    }

    fn truncate_to_width(
        text: &str,
        max_width: usize,
        metrics: crate::render::text::TextMetrics,
    ) -> String {
        metrics.clip(text, 0, max_width)
    }

    fn filter_row_offset(&self) -> u16 {
        if self.searchable {
            1
        } else {
            0
        }
    }

    fn visible_option_rows(&self) -> usize {
        let border_padding = usize::from(self.border.is_some()) * 2;
        let chrome = 2 + usize::from(self.searchable);
        self.height
            .saturating_sub(border_padding as u16)
            .saturating_sub(chrome as u16) as usize
    }
}

impl<M> Default for Select<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for Select<M> {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let is_focused = ctx.is_focused();
        let theme = ctx.theme();
        let base_style = if self.disabled {
            theme.styles.interactive_disabled
        } else {
            theme.styles.surface_elevated
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
        let disabled_style = theme.styles.interactive_disabled.to_render_style();
        let muted_style = theme.styles.text_muted.to_render_style();
        let border_style = if is_focused {
            theme.styles.border_focused.to_render_style()
        } else {
            theme.styles.border.to_render_style()
        };

        let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', field_style);

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

        let state = ctx.state_or_default::<SelectState>();
        let indicator = if state.is_open { "^" } else { "v" };
        let indicator_width = chunk.text_metrics().width(&indicator) as u16;
        let display_width = content_width.saturating_sub(indicator_width + 1) as usize;
        let display_text = self
            .selected_label(state)
            .map(|label| Self::truncate_to_width(label, display_width, chunk.text_metrics()))
            .unwrap_or_else(|| {
                Self::truncate_to_width(&self.placeholder, display_width, chunk.text_metrics())
            });
        let display_style = if self.selected_label(state).is_some() {
            field_style
        } else {
            muted_style
        };
        let _ = chunk.set_string(content_x, content_y, &display_text, display_style);
        if content_width > indicator_width {
            let indicator_x = content_x + content_width - indicator_width;
            let _ = chunk.set_string(indicator_x, content_y, indicator, display_style);
        }

        if !state.is_open || content_height <= 2 {
            return;
        }

        let mut list_y = content_y + 1;
        if self.searchable {
            let query_text = if state.search_query.is_empty() {
                "Type to filter"
            } else {
                state.search_query.as_str()
            };
            let query_style = if state.search_query.is_empty() {
                muted_style
            } else {
                field_style
            };
            let query_line = format!(
                "? {}",
                Self::truncate_to_width(
                    query_text,
                    content_width.saturating_sub(2) as usize,
                    chunk.text_metrics()
                )
            );
            let _ = chunk.fill(content_x, list_y, content_width, 1, ' ', field_style);
            let _ = chunk.set_string(content_x, list_y, &query_line, query_style);
            list_y = list_y.saturating_add(1);
        }

        let separator_y = list_y;
        let _ = chunk.fill(content_x, separator_y, content_width, 1, '-', muted_style);
        list_y = list_y.saturating_add(1);

        if self.options.is_empty() {
            let message =
                Self::truncate_to_width("No options", content_width as usize, chunk.text_metrics());
            let _ = chunk.set_string(content_x, list_y, &message, muted_style);
            return;
        }

        let filtered = self.filtered_indices(state);
        if filtered.is_empty() {
            let message = Self::truncate_to_width(
                &self.empty_search_message,
                content_width as usize,
                chunk.text_metrics(),
            );
            let _ = chunk.set_string(content_x, list_y, &message, muted_style);
            return;
        }

        let active_index = self.active_index_from_state(state, &filtered);
        let visible_rows = content_height.saturating_sub(2 + self.filter_row_offset()) as usize;
        let mut scroll_offset = state.scroll_offset.min(filtered.len().saturating_sub(1));
        if let Some(active_index) = active_index {
            scroll_offset = ensure_item_visible(scroll_offset, active_index, visible_rows);
        }

        for row in 0..visible_rows {
            let filtered_index = scroll_offset + row;
            if filtered_index >= filtered.len() {
                break;
            }

            let option_index = filtered[filtered_index];
            let option = &self.options[option_index];
            let is_active = active_index == Some(option_index);
            let is_selected =
                current_selected_value(state).as_deref() == Some(option.value.as_str());
            let style = if option.disabled {
                disabled_style
            } else if is_active {
                active_style
            } else {
                field_style
            };

            let y = list_y + row as u16;
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
            let available =
                content_width.saturating_sub(chunk.text_metrics().width(&prefix) as u16) as usize;
            let text = Self::truncate_to_width(&option.label, available, chunk.text_metrics());
            let line = format!("{prefix}{text}");
            let _ = chunk.set_string(content_x, y, &line, style);
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target || self.disabled || !self.has_enabled_options() {
            return;
        }

        let click_row = match event {
            Event::Mouse(mouse_event)
                if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left)) =>
            {
                ctx.local_mouse_position(event).map(|(_, row)| row)
            }
            _ => None,
        };
        let visible_rows = self.visible_option_rows().max(1);
        let mut emit_change = None;
        let mut did_handle = true;

        {
            let state = ctx.state_mut::<SelectState>();
            sync_state_aliases(state);
            let mut filtered = self.filtered_indices(state);

            if active_value(state).is_none() {
                if let Some(index) = self.active_index_from_state(state, &filtered) {
                    set_active_value(state, self.options[index].value.clone());
                }
            }

            let Some(mut active_index) = self.active_index_from_state(state, &filtered) else {
                return;
            };

            match event {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Down => {
                        if !state.is_open {
                            state.is_open = true;
                        } else if let Some(position) =
                            filtered.iter().position(|index| *index == active_index)
                        {
                            if position + 1 < filtered.len() {
                                active_index = filtered[position + 1];
                            }
                        }
                    }
                    KeyCode::Up => {
                        if !state.is_open {
                            state.is_open = true;
                        } else if let Some(position) =
                            filtered.iter().position(|index| *index == active_index)
                        {
                            if position > 0 {
                                active_index = filtered[position - 1];
                            }
                        }
                    }
                    KeyCode::Home if state.is_open => {
                        if let Some(index) = filtered.first().copied() {
                            active_index = index;
                        }
                    }
                    KeyCode::End if state.is_open => {
                        if let Some(index) = filtered.last().copied() {
                            active_index = index;
                        }
                    }
                    KeyCode::PageUp if state.is_open => {
                        if let Some(position) =
                            filtered.iter().position(|index| *index == active_index)
                        {
                            active_index = filtered[position.saturating_sub(visible_rows)];
                        }
                    }
                    KeyCode::PageDown if state.is_open => {
                        if let Some(position) =
                            filtered.iter().position(|index| *index == active_index)
                        {
                            let next =
                                (position + visible_rows).min(filtered.len().saturating_sub(1));
                            active_index = filtered[next];
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if !state.is_open {
                            state.is_open = true;
                        } else {
                            let value = self.options[active_index].value.clone();
                            state.selected_option = Some(value.clone());
                            state.selection.replace_selection(value.clone());
                            state.is_open = false;
                            state.search_query.clear();
                            if let Some(ref handler) = self.on_change {
                                emit_change = Some(handler(value));
                            }
                        }
                    }
                    KeyCode::Backspace if state.is_open && self.searchable => {
                        state.search_query.pop();
                        filtered = self.filtered_indices(state);
                        if let Some(index) = filtered.first().copied() {
                            active_index = index;
                        }
                    }
                    KeyCode::Char(c) if self.searchable => {
                        if !state.is_open {
                            state.is_open = true;
                        }
                        state.search_query.push(c);
                        filtered = self.filtered_indices(state);
                        if let Some(index) = filtered.first().copied() {
                            active_index = index;
                        }
                    }
                    KeyCode::Esc if state.is_open => {
                        state.is_open = false;
                        state.search_query.clear();
                    }
                    _ => {
                        did_handle = false;
                    }
                },
                Event::Mouse(mouse_event)
                    if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left)) =>
                {
                    let Some(row) = click_row else {
                        return;
                    };
                    let content_y = u16::from(self.border.is_some());
                    if row == content_y {
                        state.is_open = !state.is_open;
                    } else if state.is_open {
                        let option_start = content_y + 2 + self.filter_row_offset();
                        let Some(option_row) = row.checked_sub(option_start) else {
                            return;
                        };
                        let option_row = option_row as usize;
                        if option_row >= visible_rows {
                            return;
                        }
                        let active_position = filtered
                            .iter()
                            .position(|index| *index == active_index)
                            .unwrap_or_default();
                        let visible_offset =
                            ensure_item_visible(state.scroll_offset, active_position, visible_rows);
                        let filtered_position = visible_offset + option_row;
                        let Some(index) = filtered.get(filtered_position).copied() else {
                            return;
                        };
                        active_index = index;
                        let value = self.options[active_index].value.clone();
                        state.selected_option = Some(value.clone());
                        state.selection.replace_selection(value.clone());
                        state.is_open = false;
                        state.search_query.clear();
                        if let Some(ref handler) = self.on_change {
                            emit_change = Some(handler(value));
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }

            set_active_value(state, self.options[active_index].value.clone());
            state.scroll_offset = ensure_item_visible(
                state.scroll_offset,
                filtered
                    .iter()
                    .position(|index| *index == active_index)
                    .unwrap_or_default(),
                self.visible_option_rows().max(1),
            );
        }

        if did_handle {
            ctx.set_handled();
        }
        if let Some(message) = emit_change {
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
        if self.disabled || !self.has_enabled_options() {
            FocusConfig::None
        } else {
            FocusConfig::Composite
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a new select widget.
pub fn select<M>() -> Select<M> {
    Select::new()
}

fn active_value(state: &SelectState) -> Option<String> {
    state
        .selection
        .cursor
        .clone()
        .or_else(|| state.active_option.clone())
}

fn current_selected_value(state: &SelectState) -> Option<String> {
    state
        .selection
        .primary_selected()
        .map(str::to_owned)
        .or_else(|| state.selected_option.clone())
}

fn set_active_value(state: &mut SelectState, value: String) {
    state.active_option = Some(value.clone());
    state.selection.set_cursor(Some(value));
}

fn sync_state_aliases(state: &mut SelectState) {
    if state.selection.cursor.is_none() {
        state.selection.cursor = state.active_option.clone();
    } else if state.active_option.is_none() {
        state.active_option = state.selection.cursor.clone();
    }

    if state.selection.selected.is_empty() {
        if let Some(selected) = state.selected_option.clone() {
            state.selection.replace_selection(selected);
        }
    } else if state.selected_option.is_none() {
        state.selected_option = state.selection.primary_selected().map(str::to_owned);
    }
}

impl<M> Select<M> {
    fn preferred_width(&self) -> u16 {
        let widest_label = self
            .options
            .iter()
            .map(|option| option.label.width() as u16)
            .max()
            .unwrap_or(16);
        widest_label
            .max(self.placeholder.width() as u16)
            .saturating_add(6)
            .saturating_add(if self.border.is_some() { 2 } else { 0 })
    }
}

pub(crate) fn match_search_score(
    haystack: &str,
    needle: &str,
    mode: SelectSearchMode,
) -> Option<usize> {
    let haystack = haystack.to_lowercase();
    let needle = needle.trim().to_lowercase();

    if needle.is_empty() {
        return Some(0);
    }

    match mode {
        SelectSearchMode::Contains => haystack.find(&needle),
        SelectSearchMode::Fuzzy => {
            let mut score = 0usize;
            let mut search_start = 0usize;

            for ch in needle.chars() {
                let slice = &haystack[search_start..];
                let offset = slice.find(ch)?;
                score += offset;
                search_start += offset + ch.len_utf8();
            }

            Some(score)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{match_search_score, SelectSearchMode};

    #[test]
    fn fuzzy_matching_prefers_tighter_hits() {
        let exactish = match_search_score("deploy-preview", "dp", SelectSearchMode::Fuzzy);
        let loose = match_search_score("daily platform", "dp", SelectSearchMode::Fuzzy);

        assert!(exactish.is_some());
        assert!(loose.is_some());
        assert!(exactish < loose);
    }

    #[test]
    fn contains_matching_is_case_insensitive() {
        assert_eq!(
            match_search_score("Production", "duct", SelectSearchMode::Contains),
            Some(3)
        );
        assert_eq!(
            match_search_score("Production", "xyz", SelectSearchMode::Contains),
            None
        );
    }
}
