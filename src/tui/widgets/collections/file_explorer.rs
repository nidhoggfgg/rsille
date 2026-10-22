//! File explorer widget with tree navigation.

use crate::render::text::TextWidth;
use rustc_hash::FxHashSet;

use crate::tui::event::{Event, KeyCode};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::border_renderer;
use crate::tui::layout::{ensure_item_visible, Constraints};
use crate::tui::style::{BorderStyle, Style};
use crate::tui::widget::{EventCtx, EventPhase, RenderCtx, Widget};

use super::selection::{SelectionMode, SelectionState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExplorerItemKind {
    File,
    Directory,
    LazyDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileExplorerItem {
    pub id: String,
    pub label: String,
    pub kind: FileExplorerItemKind,
    pub children: Vec<FileExplorerItem>,
    pub disabled: bool,
}

impl FileExplorerItem {
    pub fn file(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(id, label, FileExplorerItemKind::File)
    }

    pub fn directory(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(id, label, FileExplorerItemKind::Directory)
    }

    pub fn lazy_directory(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(id, label, FileExplorerItemKind::LazyDirectory)
    }

    pub fn child(mut self, child: FileExplorerItem) -> Self {
        self.children.push(child);
        self
    }

    pub fn children<I>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = FileExplorerItem>,
    {
        self.children.extend(children);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn new(id: impl Into<String>, label: impl Into<String>, kind: FileExplorerItemKind) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind,
            children: Vec::new(),
            disabled: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FileExplorerState {
    pub active_item: Option<String>,
    pub expanded_items: FxHashSet<String>,
    pub selection: SelectionState,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
struct VisibleFileRow {
    id: String,
    label: String,
    depth: usize,
    parent_id: Option<String>,
    kind: FileExplorerItemKind,
    is_expanded: bool,
    disabled: bool,
}

pub struct FileExplorer<M = ()> {
    items: Vec<FileExplorerItem>,
    height: u16,
    border: Option<BorderStyle>,
    disabled: bool,
    selection_mode: SelectionMode,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(String) -> M>>,
    on_open: Option<Box<dyn Fn(String) -> M>>,
    on_load_children: Option<Box<dyn Fn(String) -> M>>,
    on_selection_change: Option<Box<dyn Fn(Vec<String>) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for FileExplorer<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileExplorer")
            .field("items", &self.items)
            .field("height", &self.height)
            .field("border", &self.border)
            .field("disabled", &self.disabled)
            .field("selection_mode", &self.selection_mode)
            .field("on_change", &self.on_change.is_some())
            .field("on_open", &self.on_open.is_some())
            .field("on_load_children", &self.on_load_children.is_some())
            .field("on_selection_change", &self.on_selection_change.is_some())
            .finish()
    }
}

impl<M> FileExplorer<M> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            height: 12,
            border: Some(BorderStyle::Single),
            disabled: false,
            selection_mode: SelectionMode::Single,
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            on_open: None,
            on_load_children: None,
            on_selection_change: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn item(mut self, item: FileExplorerItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = FileExplorerItem>,
    {
        self.items.extend(items);
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

    pub fn on_open<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_open = Some(Box::new(handler));
        self
    }

    pub fn on_load_children<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_load_children = Some(Box::new(handler));
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

    fn has_enabled_in_subtree(item: &FileExplorerItem) -> bool {
        !item.disabled || item.children.iter().any(Self::has_enabled_in_subtree)
    }

    fn visible_rows(&self) -> usize {
        let border_padding = usize::from(self.border.is_some()) * 2;
        self.height.saturating_sub(border_padding as u16) as usize
    }

    fn flatten_visible_rows(&self, expanded: &FxHashSet<String>) -> Vec<VisibleFileRow> {
        let mut rows = Vec::new();
        for item in &self.items {
            Self::collect_rows(item, 0, None, expanded, &mut rows);
        }
        rows
    }

    fn collect_rows(
        item: &FileExplorerItem,
        depth: usize,
        parent_id: Option<&str>,
        expanded: &FxHashSet<String>,
        rows: &mut Vec<VisibleFileRow>,
    ) {
        let is_expanded = expanded.contains(&item.id);
        rows.push(VisibleFileRow {
            id: item.id.clone(),
            label: item.label.clone(),
            depth,
            parent_id: parent_id.map(str::to_owned),
            kind: item.kind,
            is_expanded,
            disabled: item.disabled,
        });

        if is_expanded {
            for child in &item.children {
                Self::collect_rows(child, depth + 1, Some(&item.id), expanded, rows);
            }
        }
    }

    fn active_index_from_state(
        &self,
        state: &FileExplorerState,
        rows: &[VisibleFileRow],
    ) -> Option<usize> {
        state
            .selection
            .cursor()
            .or(state.active_item.as_deref())
            .and_then(|id| rows.iter().position(|row| row.id == id && !row.disabled))
            .or_else(|| rows.iter().position(|row| !row.disabled))
    }

    fn next_enabled_index(rows: &[VisibleFileRow], current: usize) -> Option<usize> {
        rows.iter()
            .enumerate()
            .skip(current.saturating_add(1))
            .find(|(_, row)| !row.disabled)
            .map(|(index, _)| index)
    }

    fn prev_enabled_index(rows: &[VisibleFileRow], current: usize) -> Option<usize> {
        rows.iter()
            .enumerate()
            .take(current)
            .rev()
            .find(|(_, row)| !row.disabled)
            .map(|(index, _)| index)
    }

    fn render_row_text(
        row: &VisibleFileRow,
        width: usize,
        metrics: crate::render::text::TextMetrics,
    ) -> String {
        let indent = "  ".repeat(row.depth);
        let marker = match row.kind {
            FileExplorerItemKind::File => "  ",
            FileExplorerItemKind::Directory if row.is_expanded => "- ",
            FileExplorerItemKind::Directory => "+ ",
            FileExplorerItemKind::LazyDirectory if row.is_expanded => "~ ",
            FileExplorerItemKind::LazyDirectory => "+ ",
        };
        let available = width.saturating_sub(indent.width() + marker.width());
        let label = truncate_to_width(&row.label, available, metrics);
        format!("{indent}{marker}{label}")
    }
}

impl<M> Default for FileExplorer<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for FileExplorer<M> {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let theme = ctx.theme();
        let row_style = self
            .custom_style
            .map(|style| style.merge(theme.styles.surface_elevated))
            .unwrap_or(theme.styles.surface_elevated)
            .to_render_style();
        let active_style = if ctx.is_focused() {
            self.custom_focus_style
                .unwrap_or(theme.styles.selected_focused)
        } else {
            theme.styles.selected
        }
        .to_render_style();
        let selected_style = theme.styles.selected.to_render_style();
        let disabled_style = theme.styles.interactive_disabled.to_render_style();
        let muted_style = theme.styles.text_muted.to_render_style();
        let border_style = if ctx.is_focused() {
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
                (1, 1, area.width() - 2, area.height() - 2)
            } else {
                (0, 0, area.width(), area.height())
            };

        let state = ctx.state_or_default::<FileExplorerState>();
        let rows = self.flatten_visible_rows(&state.expanded_items);
        if rows.is_empty() {
            let _ = chunk.set_string(content_x, content_y, "No files", muted_style);
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
        let mut emit_open = None;
        let mut emit_load = None;
        let mut emit_selection = None;

        let next_active_id = {
            let state = ctx.state_mut::<FileExplorerState>();
            if state.selection.cursor.is_none() {
                state.selection.cursor = state.active_item.clone();
            }
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
                    active_index = rows
                        .iter()
                        .position(|row| !row.disabled)
                        .unwrap_or(active_index);
                    moved = true;
                }
                KeyCode::End => {
                    active_index = rows
                        .iter()
                        .rposition(|row| !row.disabled)
                        .unwrap_or(active_index);
                    moved = true;
                }
                KeyCode::PageUp => {
                    active_index = active_index.saturating_sub(visible_rows);
                    moved = true;
                }
                KeyCode::PageDown => {
                    active_index = (active_index + visible_rows).min(rows.len().saturating_sub(1));
                    moved = true;
                }
                KeyCode::Right => {
                    let row = &rows[active_index];
                    match row.kind {
                        FileExplorerItemKind::Directory => {
                            state.expanded_items.insert(row.id.clone());
                        }
                        FileExplorerItemKind::LazyDirectory => {
                            state.expanded_items.insert(row.id.clone());
                            emit_load = self
                                .on_load_children
                                .as_ref()
                                .map(|handler| handler(row.id.clone()));
                        }
                        FileExplorerItemKind::File => {}
                    }
                }
                KeyCode::Left => {
                    let row = &rows[active_index];
                    if matches!(
                        row.kind,
                        FileExplorerItemKind::Directory | FileExplorerItemKind::LazyDirectory
                    ) && row.is_expanded
                    {
                        state.expanded_items.remove(&row.id);
                    } else if let Some(parent_id) = row.parent_id.as_deref() {
                        if let Some(index) =
                            rows.iter().position(|candidate| candidate.id == parent_id)
                        {
                            active_index = index;
                            moved = true;
                        }
                    }
                }
                KeyCode::Char(' ') if self.selection_mode.is_multiple() => {
                    let row = &rows[active_index];
                    state.selection.toggle(&row.id);
                    emit_selection = self
                        .on_selection_change
                        .as_ref()
                        .map(|handler| handler(state.selection.selected.clone()));
                }
                KeyCode::Enter => {
                    let row = &rows[active_index];
                    match row.kind {
                        FileExplorerItemKind::File => {
                            state.selection.replace_selection(row.id.clone());
                            emit_open =
                                self.on_open.as_ref().map(|handler| handler(row.id.clone()));
                        }
                        FileExplorerItemKind::Directory => {
                            if row.is_expanded {
                                state.expanded_items.remove(&row.id);
                            } else {
                                state.expanded_items.insert(row.id.clone());
                            }
                        }
                        FileExplorerItemKind::LazyDirectory => {
                            state.expanded_items.insert(row.id.clone());
                            emit_load = self
                                .on_load_children
                                .as_ref()
                                .map(|handler| handler(row.id.clone()));
                        }
                    }
                }
                _ => return,
            }

            let rows = self.flatten_visible_rows(&state.expanded_items);
            if rows.is_empty() {
                state.active_item = None;
                state.selection.set_cursor(None);
                return;
            }
            active_index = active_index.min(rows.len().saturating_sub(1));
            state.active_item = Some(rows[active_index].id.clone());
            state.selection.set_cursor(state.active_item.clone());
            state.scroll_offset =
                ensure_item_visible(state.scroll_offset, active_index, visible_rows);
            state.active_item.clone()
        };

        ctx.set_handled();
        if moved {
            if let (Some(active_id), Some(handler)) = (next_active_id, self.on_change.as_ref()) {
                ctx.emit(handler(active_id));
            }
        }
        if let Some(message) = emit_open {
            ctx.emit(message);
        }
        if let Some(message) = emit_load {
            ctx.emit(message);
        }
        if let Some(message) = emit_selection {
            ctx.emit(message);
        }
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: 24,
            max_width: None,
            min_height: self.height,
            max_height: Some(self.height),
            flex: Some(1.0),
        }
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

fn truncate_to_width(
    text: &str,
    max_width: usize,
    metrics: crate::render::text::TextMetrics,
) -> String {
    metrics.clip(text, 0, max_width)
}

pub fn file_explorer<M>() -> FileExplorer<M> {
    FileExplorer::new()
}
