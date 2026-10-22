//! Tree widget with expandable nodes and internal navigation.

use render::text::TextWidth;
use rustc_hash::FxHashSet;

use crate::event::{Event, KeyCode};
use crate::focus::FocusConfig;
use crate::layout::border_renderer;
use crate::layout::{ensure_item_visible, Constraints, LayoutStyle};
use crate::style::{BorderStyle, Style};
use crate::widget::{EventCtx, EventPhase, RenderCtx, Widget};

use super::selection::{SelectionMode, SelectionState};

/// A node in the tree widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeItem {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeItem>,
    pub disabled: bool,
}

impl TreeItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            disabled: false,
        }
    }

    pub fn child(mut self, child: TreeItem) -> Self {
        self.children.push(child);
        self
    }

    pub fn children<I>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = TreeItem>,
    {
        self.children.extend(children);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Persistent tree widget state stored in the widget store.
#[derive(Debug, Clone, Default)]
pub struct TreeState {
    pub active_item: Option<String>,
    pub expanded_items: FxHashSet<String>,
    pub selection: SelectionState,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
struct VisibleTreeRow {
    id: String,
    label: String,
    depth: usize,
    parent_id: Option<String>,
    has_children: bool,
    is_expanded: bool,
    disabled: bool,
}

/// Focusable tree widget with expandable nodes and keyboard navigation.
pub struct Tree<M = ()> {
    items: Vec<TreeItem>,
    height: u16,
    border: Option<BorderStyle>,
    disabled: bool,
    empty_message: String,
    selection_mode: SelectionMode,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(String) -> M>>,
    on_submit: Option<Box<dyn Fn(String) -> M>>,
    on_selection_change: Option<Box<dyn Fn(Vec<String>) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Tree<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("items", &self.items)
            .field("height", &self.height)
            .field("border", &self.border)
            .field("disabled", &self.disabled)
            .field("empty_message", &self.empty_message)
            .field("selection_mode", &self.selection_mode)
            .field("on_change", &self.on_change.is_some())
            .field("on_submit", &self.on_submit.is_some())
            .field("on_selection_change", &self.on_selection_change.is_some())
            .finish()
    }
}

impl<M> Tree<M> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            height: 10,
            border: Some(BorderStyle::Single),
            disabled: false,
            empty_message: "No items".to_owned(),
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

    pub fn item(mut self, item: TreeItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = TreeItem>,
    {
        self.items.extend(items);
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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = message.into();
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
        self.items.iter().any(Self::has_enabled_in_subtree)
    }

    fn has_enabled_in_subtree(item: &TreeItem) -> bool {
        !item.disabled || item.children.iter().any(Self::has_enabled_in_subtree)
    }

    fn visible_rows(&self) -> usize {
        let border_padding = usize::from(self.border.is_some()) * 2;
        self.height.saturating_sub(border_padding as u16) as usize
    }

    fn flatten_visible_rows(&self, expanded: &FxHashSet<String>) -> Vec<VisibleTreeRow> {
        let mut rows = Vec::new();
        for item in &self.items {
            Self::collect_rows(item, 0, None, expanded, &mut rows);
        }
        rows
    }

    fn collect_rows(
        item: &TreeItem,
        depth: usize,
        parent_id: Option<&str>,
        expanded: &FxHashSet<String>,
        rows: &mut Vec<VisibleTreeRow>,
    ) {
        let is_expanded = expanded.contains(&item.id);
        rows.push(VisibleTreeRow {
            id: item.id.clone(),
            label: item.label.clone(),
            depth,
            parent_id: parent_id.map(str::to_owned),
            has_children: !item.children.is_empty(),
            is_expanded,
            disabled: item.disabled,
        });

        if is_expanded {
            for child in &item.children {
                Self::collect_rows(child, depth + 1, Some(&item.id), expanded, rows);
            }
        }
    }

    fn active_index_from_state(&self, state: &TreeState, rows: &[VisibleTreeRow]) -> Option<usize> {
        active_item_id(state)
            .as_deref()
            .and_then(|id| rows.iter().position(|row| row.id == id && !row.disabled))
            .or_else(|| rows.iter().position(|row| !row.disabled))
    }

    fn next_enabled_index(rows: &[VisibleTreeRow], current: usize) -> Option<usize> {
        rows.iter()
            .enumerate()
            .skip(current.saturating_add(1))
            .find(|(_, row)| !row.disabled)
            .map(|(index, _)| index)
    }

    fn prev_enabled_index(rows: &[VisibleTreeRow], current: usize) -> Option<usize> {
        rows.iter()
            .enumerate()
            .take(current)
            .rev()
            .find(|(_, row)| !row.disabled)
            .map(|(index, _)| index)
    }

    fn truncate_to_width(
        text: &str,
        max_width: usize,
        metrics: render::text::TextMetrics,
    ) -> String {
        metrics.clip(text, 0, max_width)
    }

    fn render_row_text(
        row: &VisibleTreeRow,
        width: usize,
        metrics: render::text::TextMetrics,
    ) -> String {
        let indent = "  ".repeat(row.depth);
        let marker = if row.has_children {
            if row.is_expanded {
                "- "
            } else {
                "+ "
            }
        } else {
            "  "
        };
        let available = width.saturating_sub(indent.width() + marker.width());
        let label = Self::truncate_to_width(&row.label, available, metrics);
        format!("{indent}{marker}{label}")
    }
}

impl<M> Default for Tree<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for Tree<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
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
        let row_style = self
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
        let selected_style = theme.styles.selected.to_render_style();
        let disabled_style = theme.styles.interactive_disabled.to_render_style();
        let muted_style = theme.styles.text_muted.to_render_style();
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

        let state = ctx.state_or_default::<TreeState>();
        let rows = self.flatten_visible_rows(&state.expanded_items);

        if rows.is_empty() {
            let message = Self::truncate_to_width(
                &self.empty_message,
                content_width as usize,
                chunk.text_metrics(),
            );
            let _ = chunk.set_string(content_x, content_y, &message, muted_style);
            return;
        }

        let active_index = self.active_index_from_state(state, &rows);
        let visible_rows = content_height as usize;
        let mut scroll_offset = state.scroll_offset.min(rows.len().saturating_sub(1));
        if let Some(active_index) = active_index {
            scroll_offset = ensure_item_visible(scroll_offset, active_index, visible_rows);
        }

        for row in 0..visible_rows {
            let row_index = scroll_offset + row;
            if row_index >= rows.len() {
                break;
            }

            let row_item = &rows[row_index];
            let is_active = active_index == Some(row_index);
            let is_selected = state.selection.is_selected(&row_item.id);
            let style = if row_item.disabled {
                disabled_style
            } else if is_active {
                active_style
            } else if is_selected {
                selected_style
            } else {
                row_style
            };

            let y = content_y + row as u16;
            let _ = chunk.fill(content_x, y, content_width, 1, ' ', style);
            let text =
                Self::render_row_text(row_item, content_width as usize, chunk.text_metrics());
            let _ = chunk.set_string(content_x, y, &text, style);
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target || self.disabled || !self.has_enabled_items() {
            return;
        }

        let Event::Key(key_event) = event else {
            return;
        };

        let visible_rows = self.visible_rows().max(1);
        let mut moved = false;
        let mut emit_submit = None;

        let next_active_id = {
            let state = ctx.state_mut::<TreeState>();
            let rows = self.flatten_visible_rows(&state.expanded_items);
            let Some(mut active_index) = self.active_index_from_state(state, &rows) else {
                return;
            };

            match key_event.code {
                KeyCode::Up => {
                    if let Some(index) = Self::prev_enabled_index(&rows, active_index) {
                        active_index = index;
                        moved = true;
                    }
                }
                KeyCode::Down => {
                    if let Some(index) = Self::next_enabled_index(&rows, active_index) {
                        active_index = index;
                        moved = true;
                    }
                }
                KeyCode::Home => {
                    if let Some(index) = rows.iter().position(|row| !row.disabled) {
                        active_index = index;
                        moved = true;
                    }
                }
                KeyCode::End => {
                    if let Some(index) = rows.iter().rposition(|row| !row.disabled) {
                        active_index = index;
                        moved = true;
                    }
                }
                KeyCode::PageUp => {
                    for _ in 0..visible_rows {
                        if let Some(index) = Self::prev_enabled_index(&rows, active_index) {
                            active_index = index;
                            moved = true;
                        } else {
                            break;
                        }
                    }
                }
                KeyCode::PageDown => {
                    for _ in 0..visible_rows {
                        if let Some(index) = Self::next_enabled_index(&rows, active_index) {
                            active_index = index;
                            moved = true;
                        } else {
                            break;
                        }
                    }
                }
                KeyCode::Right => {
                    let row = &rows[active_index];
                    if row.has_children && !row.is_expanded {
                        state.expanded_items.insert(row.id.clone());
                    } else if row.has_children {
                        if let Some(index) = rows
                            .iter()
                            .enumerate()
                            .skip(active_index + 1)
                            .find(|(_, candidate)| {
                                candidate.parent_id.as_deref() == Some(row.id.as_str())
                                    && !candidate.disabled
                            })
                            .map(|(index, _)| index)
                        {
                            active_index = index;
                            moved = true;
                        }
                    }
                }
                KeyCode::Left => {
                    let row = &rows[active_index];
                    if row.has_children && row.is_expanded {
                        state.expanded_items.remove(&row.id);
                    } else if let Some(parent_id) = row.parent_id.as_deref() {
                        if let Some(index) = rows
                            .iter()
                            .position(|candidate| candidate.id == parent_id && !candidate.disabled)
                        {
                            active_index = index;
                            moved = true;
                        }
                    }
                }
                KeyCode::Char(' ') if self.selection_mode.is_multiple() => {
                    let row = &rows[active_index];
                    state.selection.toggle(&row.id);
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
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let row = &rows[active_index];
                    if row.has_children {
                        if row.is_expanded {
                            state.expanded_items.remove(&row.id);
                        } else {
                            state.expanded_items.insert(row.id.clone());
                        }
                        ctx.set_handled();
                        return;
                    }

                    if let Some(ref handler) = self.on_submit {
                        emit_submit = Some(handler(row.id.clone()));
                    }
                    if self.selection_mode == SelectionMode::Single {
                        state.selection.replace_selection(row.id.clone());
                    }
                    ctx.set_handled();
                    if let Some(message) = emit_submit {
                        ctx.emit(message);
                    }
                    return;
                }
                _ => return,
            }

            let rows = self.flatten_visible_rows(&state.expanded_items);
            if rows.is_empty() {
                state.active_item = None;
                return;
            }

            if active_index >= rows.len() {
                active_index = rows.len() - 1;
            }

            if rows[active_index].disabled {
                if let Some(index) = Self::next_enabled_index(&rows, active_index)
                    .or_else(|| Self::prev_enabled_index(&rows, active_index))
                {
                    active_index = index;
                }
            }

            state.active_item = Some(rows[active_index].id.clone());
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

/// Create a new tree widget.
pub fn tree<M>() -> Tree<M> {
    Tree::new()
}

fn active_item_id(state: &TreeState) -> Option<String> {
    state
        .selection
        .cursor
        .clone()
        .or_else(|| state.active_item.clone())
}

impl<M> Tree<M> {
    fn preferred_width(&self) -> u16 {
        fn deepest_width(items: &[TreeItem], depth: usize) -> u16 {
            items
                .iter()
                .map(|item| {
                    let own = (depth * 2) as u16 + item.label.width() as u16 + 2;
                    own.max(deepest_width(&item.children, depth + 1))
                })
                .max()
                .unwrap_or(12)
        }

        deepest_width(&self.items, 0).saturating_add(if self.border.is_some() { 2 } else { 0 })
    }
}
