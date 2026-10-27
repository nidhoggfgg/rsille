//! Data table widget with internal row navigation.

use crate::render::text::TextWidth;

use crate::tui::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::border_renderer;
use crate::tui::layout::{ensure_item_visible, Constraints, LayoutStyle};
use crate::tui::style::{BorderStyle, Style};
use crate::tui::widget::{EventCtx, EventPhase, Lifecycle, RenderCtx, StateCtx, Widget};
use crate::tui::widgets::pointer;

use super::selection::{SelectionMode, SelectionState};

/// Column alignment within a data table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Data table column definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableColumn {
    pub id: String,
    pub title: String,
    pub width: u16,
    pub align: TableAlign,
    pub sortable: bool,
    pub filterable: bool,
    pub visible: bool,
}

impl DataTableColumn {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        let id = title.to_lowercase().replace(char::is_whitespace, "-");
        let width = title.width().max(8) as u16;
        Self {
            id,
            title,
            width,
            align: TableAlign::Left,
            sortable: false,
            filterable: false,
            visible: true,
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = width.max(3);
        self
    }

    pub fn align(mut self, align: TableAlign) -> Self {
        self.align = align;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

/// A single row in the data table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableRow {
    pub id: String,
    pub cells: Vec<String>,
    pub disabled: bool,
}

impl DataTableRow {
    pub fn new(id: impl Into<String>, cells: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            id: id.into(),
            cells: cells.into_iter().map(Into::into).collect(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Persistent data table state stored in the widget store.
#[derive(Debug, Clone, Default)]
pub struct DataTableState {
    /// Active resize gesture: column id, initial pointer column, and initial width.
    pub resizing_column: Option<(String, u16, u16)>,
    pub active_row: Option<String>,
    pub active_column: usize,
    pub selection: SelectionState,
    pub column_widths: Vec<(String, u16)>,
    pub scroll_offset: usize,
    #[doc(hidden)]
    pub controlled_active: Option<Option<String>>,
    #[doc(hidden)]
    pub active_row_initialized: bool,
    #[doc(hidden)]
    pub controlled_selection: Option<Vec<String>>,
}

/// Data table sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTableSortDirection {
    Asc,
    Desc,
}

/// Controlled sort state for a data table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableSort {
    pub column_id: String,
    pub direction: DataTableSortDirection,
}

/// Keyboard navigation model for a data table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DataTableNavigationMode {
    #[default]
    Row,
    Cell,
}

impl DataTableSort {
    pub fn new(column_id: impl Into<String>, direction: DataTableSortDirection) -> Self {
        Self {
            column_id: column_id.into(),
            direction,
        }
    }
}

/// Focusable data table widget with internal row navigation.
pub struct DataTable<M = ()> {
    columns: Vec<DataTableColumn>,
    rows: Vec<DataTableRow>,
    height: u16,
    border: Option<BorderStyle>,
    disabled: bool,
    empty_message: String,
    filter_query: Option<String>,
    sort: Option<DataTableSort>,
    hidden_columns: Vec<String>,
    sticky_header: bool,
    navigation_mode: DataTableNavigationMode,
    selection_mode: SelectionMode,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    active_row: Option<Option<String>>,
    selected_ids: Option<Vec<String>>,
    default_active_row: Option<Option<String>>,
    default_selected_ids: Vec<String>,
    on_active_row_change: Option<Box<dyn Fn(Option<String>) -> M>>,
    on_change: Option<Box<dyn Fn(String) -> M>>,
    on_submit: Option<Box<dyn Fn(String) -> M>>,
    on_cell_change: Option<Box<dyn Fn(String, String) -> M>>,
    on_selection_change: Option<Box<dyn Fn(Vec<String>) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for DataTable<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataTable")
            .field("columns", &self.columns)
            .field("rows", &self.rows)
            .field("height", &self.height)
            .field("border", &self.border)
            .field("disabled", &self.disabled)
            .field("empty_message", &self.empty_message)
            .field("filter_query", &self.filter_query)
            .field("sort", &self.sort)
            .field("hidden_columns", &self.hidden_columns)
            .field("sticky_header", &self.sticky_header)
            .field("navigation_mode", &self.navigation_mode)
            .field("selection_mode", &self.selection_mode)
            .field("on_change", &self.on_change.is_some())
            .field("on_submit", &self.on_submit.is_some())
            .field("on_cell_change", &self.on_cell_change.is_some())
            .field("on_selection_change", &self.on_selection_change.is_some())
            .finish()
    }
}

impl<M: 'static> DataTable<M> {
    fn handle_input(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::tui::event::is_key_release(event) {
            return;
        }
        if ctx.phase() != EventPhase::Target {
            return;
        }
        if self.disabled || matches!(event, Event::FocusLost) {
            ctx.state_mut::<DataTableState>().resizing_column = None;
            return;
        }
        if matches!(event, Event::Mouse(_)) {
            self.handle_pointer(event, ctx);
            return;
        }
        if !self.has_enabled_rows() {
            return;
        }
        let Event::Key(key_event) = event else {
            return;
        };

        let visible_rows = pointer::visible_rows(
            ctx,
            self.border.is_some(),
            self.height,
            if self.sticky_header { 2 } else { 0 },
        )
        .max(1);
        let mut moved = false;
        let mut emit_submit = None;
        let mut emit_cell_change = None;

        let next_active_id = {
            let state = ctx.state_mut::<DataTableState>();
            sync_state_aliases(state);
            let visible_indices = self.visible_row_indices();
            let visible_columns = self.visible_columns();
            let Some(mut active_position) = self.navigation_index(state, &visible_indices) else {
                return;
            };
            if visible_columns.is_empty() {
                return;
            }
            state.active_column = state
                .active_column
                .min(visible_columns.len().saturating_sub(1));

            let empty_cursor = state.active_row.is_none()
                && (self.active_row.is_some() || state.active_row_initialized);
            match key_event.code {
                KeyCode::Down if empty_cursor => {
                    active_position = self.navigation_index(state, &visible_indices).unwrap();
                    moved = true;
                }
                KeyCode::Up if empty_cursor => {
                    active_position = visible_indices
                        .iter()
                        .rposition(|i| !self.rows[*i].disabled)
                        .unwrap();
                    moved = true;
                }
                KeyCode::Enter | KeyCode::Char(' ') if empty_cursor => return,
                KeyCode::Up => {
                    if let Some(position) = (0..active_position)
                        .rev()
                        .find(|i| !self.rows[visible_indices[*i]].disabled)
                    {
                        active_position = position;
                        moved = true;
                    }
                }
                KeyCode::Down => {
                    if let Some(position) = (active_position + 1..visible_indices.len())
                        .find(|i| !self.rows[visible_indices[*i]].disabled)
                    {
                        active_position = position;
                        moved = true;
                    }
                }
                KeyCode::Left if self.navigation_mode == DataTableNavigationMode::Cell => {
                    if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        let column = visible_columns[state.active_column].1;
                        let current_width = Self::column_width_override(state, &column.id)
                            .map(|(_, width)| *width)
                            .unwrap_or(column.width);
                        Self::set_column_width(state, &column.id, current_width.saturating_sub(1));
                    } else if state.active_column > 0 {
                        state.active_column -= 1;
                        moved = true;
                    }
                }
                KeyCode::Right if self.navigation_mode == DataTableNavigationMode::Cell => {
                    if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        let column = visible_columns[state.active_column].1;
                        let current_width = Self::column_width_override(state, &column.id)
                            .map(|(_, width)| *width)
                            .unwrap_or(column.width);
                        Self::set_column_width(state, &column.id, current_width.saturating_add(1));
                    } else if state.active_column + 1 < visible_columns.len() {
                        state.active_column += 1;
                        moved = true;
                    }
                }
                KeyCode::Home => {
                    active_position = visible_indices
                        .iter()
                        .position(|i| !self.rows[*i].disabled)
                        .unwrap();
                    moved = true;
                }
                KeyCode::End => {
                    active_position = visible_indices
                        .iter()
                        .rposition(|i| !self.rows[*i].disabled)
                        .unwrap();
                    moved = true;
                }
                KeyCode::PageUp => {
                    active_position = active_position.saturating_sub(visible_rows);
                    moved = true;
                }
                KeyCode::PageDown => {
                    active_position = (active_position + visible_rows)
                        .min(visible_indices.len().saturating_sub(1));
                    moved = true;
                }
                KeyCode::Enter => {
                    let active_id = self.rows[visible_indices[active_position]].id.clone();
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
                    let active_id = self.rows[visible_indices[active_position]].id.clone();
                    state.selection.toggle(&active_id);
                    ctx.set_handled();
                    return;
                }
                _ => return,
            }

            if self.rows[visible_indices[active_position]].disabled {
                active_position = (active_position..visible_indices.len())
                    .find(|i| !self.rows[visible_indices[*i]].disabled)
                    .or_else(|| {
                        (0..active_position)
                            .rev()
                            .find(|i| !self.rows[visible_indices[*i]].disabled)
                    })
                    .unwrap();
            }
            let active_id = self.rows[visible_indices[active_position]].id.clone();
            set_active_row(state, active_id);
            state.scroll_offset =
                ensure_item_visible(state.scroll_offset, active_position, visible_rows);
            if moved && self.navigation_mode == DataTableNavigationMode::Cell {
                let row_id = self.rows[visible_indices[active_position]].id.clone();
                let column_id = visible_columns[state.active_column].1.id.clone();
                emit_cell_change = self
                    .on_cell_change
                    .as_ref()
                    .map(|handler| handler(row_id, column_id));
            }
            active_row_id(state)
        };

        if moved {
            ctx.set_handled();
            if let (Some(active_id), Some(handler)) = (next_active_id, self.on_change.as_ref()) {
                ctx.emit(handler(active_id));
            }
            if let Some(message) = emit_cell_change {
                ctx.emit(message);
            }
        }
    }
}

impl<M> DataTable<M> {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            height: 10,
            border: Some(BorderStyle::Single),
            disabled: false,
            empty_message: "No rows".to_owned(),
            filter_query: None,
            sort: None,
            hidden_columns: Vec::new(),
            sticky_header: true,
            navigation_mode: DataTableNavigationMode::Row,
            selection_mode: SelectionMode::Single,
            custom_style: None,
            custom_focus_style: None,
            active_row: None,
            selected_ids: None,
            default_active_row: None,
            default_selected_ids: Vec::new(),
            on_active_row_change: None,
            on_change: None,
            on_submit: None,
            on_cell_change: None,
            on_selection_change: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn column(mut self, column: DataTableColumn) -> Self {
        self.columns.push(column);
        self
    }

    pub fn columns<I>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = DataTableColumn>,
    {
        self.columns.extend(columns);
        self
    }

    pub fn row(mut self, row: DataTableRow) -> Self {
        self.rows.push(row);
        self
    }

    pub fn rows<I>(mut self, rows: I) -> Self
    where
        I: IntoIterator<Item = DataTableRow>,
    {
        self.rows.extend(rows);
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

    pub fn filter_query(mut self, query: impl Into<String>) -> Self {
        self.filter_query = Some(query.into());
        self
    }

    pub fn filter_query_opt(mut self, query: Option<String>) -> Self {
        self.filter_query = query;
        self
    }

    pub fn sort(mut self, sort: DataTableSort) -> Self {
        self.sort = Some(sort);
        self
    }

    pub fn sort_opt(mut self, sort: Option<DataTableSort>) -> Self {
        self.sort = sort;
        self
    }

    pub fn hidden_column(mut self, column_id: impl Into<String>) -> Self {
        self.hidden_columns.push(column_id.into());
        self
    }

    pub fn hidden_columns<I, S>(mut self, column_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.hidden_columns
            .extend(column_ids.into_iter().map(Into::into));
        self
    }

    pub fn sticky_header(mut self, sticky: bool) -> Self {
        self.sticky_header = sticky;
        self
    }

    pub fn navigation_mode(mut self, mode: DataTableNavigationMode) -> Self {
        self.navigation_mode = mode;
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

    /// Bind the logical current row. `None` explicitly clears the cursor.
    /// Hidden or invalid ids are retained; navigation falls back to the first
    /// enabled visible row. With no cursor, Down/Up chooses the first/last row
    /// and Enter does nothing. Sync never emits change callbacks.
    pub fn active_row(mut self, id: Option<String>) -> Self {
        self.active_row = Some(id);
        self
    }

    pub fn default_active_row(mut self, id: Option<String>) -> Self {
        self.default_active_row = Some(id);
        self
    }

    /// Bind selection independently of the current row. Unknown and filtered
    /// ids survive user edits; duplicates are removed in first-occurrence order.
    pub fn selected_ids(mut self, ids: impl IntoIterator<Item = String>) -> Self {
        self.selected_ids = Some(ids.into_iter().collect());
        self
    }

    pub fn default_selected_ids(mut self, ids: impl IntoIterator<Item = String>) -> Self {
        self.default_selected_ids = ids.into_iter().collect();
        self
    }

    /// Emitted only for semantic user changes. The legacy `on_change` retains
    /// its notification timing; when both are set, both receive row changes.
    pub fn on_active_row_change(mut self, handler: impl Fn(Option<String>) -> M + 'static) -> Self {
        self.on_active_row_change = Some(Box::new(handler));
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

    pub fn on_cell_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(String, String) -> M + 'static,
    {
        self.on_cell_change = Some(Box::new(handler));
        self
    }

    pub fn on_selection_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(Vec<String>) -> M + 'static,
    {
        self.on_selection_change = Some(Box::new(handler));
        self
    }

    fn handle_pointer(&self, event: &Event, ctx: &mut EventCtx<M>) {
        let Event::Mouse(mouse) = event else {
            return;
        };
        if mouse.kind == MouseEventKind::Up(MouseButton::Left) {
            if ctx
                .state_mut::<DataTableState>()
                .resizing_column
                .take()
                .is_some()
            {
                ctx.set_handled();
            }
            return;
        }
        if mouse.kind == MouseEventKind::Drag(MouseButton::Left) {
            let drag = ctx.state_mut::<DataTableState>().resizing_column.clone();
            if let (Some((id, start_x, width)), Some((x, _))) =
                (drag, ctx.layout_mouse_position(event))
            {
                let width = (i32::from(width) + i32::from(x) - i32::from(start_x))
                    .clamp(3, u16::MAX as i32) as u16;
                Self::set_column_width(ctx.state_mut::<DataTableState>(), &id, width);
                ctx.set_handled();
            }
            return;
        }
        let Some(bounds) = ctx.bounds() else {
            return;
        };
        let border = self.border.is_some();
        let header = if self.sticky_header { 2 } else { 0 };
        let visible = pointer::visible_rows(ctx, border, self.height, header);
        let columns = self.visible_columns();
        if columns.is_empty() {
            return;
        }
        let content_width = bounds.width().saturating_sub(u16::from(border) * 2);
        let position = pointer::content_position(ctx, event, border, 0);
        // The header's column separators serve as resize handles.
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            ctx.state_mut::<DataTableState>().resizing_column = None;
            if let Some((x, 0)) = position.filter(|_| self.sticky_header) {
                let widths = self.compute_column_widths(
                    &columns,
                    content_width,
                    ctx.state_mut::<DataTableState>(),
                );
                let mut edge = 0usize;
                for (index, width) in widths
                    .iter()
                    .enumerate()
                    .take(widths.len().saturating_sub(1))
                {
                    edge += *width as usize;
                    if x == edge {
                        let Some((start_x, _)) = ctx.layout_mouse_position(event) else {
                            return;
                        };
                        ctx.state_mut::<DataTableState>().resizing_column =
                            Some((columns[index].1.id.clone(), start_x, *width));
                        ctx.set_handled();
                        return;
                    }
                    edge += 1;
                }
            }
        }
        if visible == 0 {
            return;
        }
        let (id, column_id) = {
            let state = ctx.state_mut::<DataTableState>();
            sync_state_aliases(state);
            let rows = self.visible_row_indices();
            let Some(active) = self.navigation_index(state, &rows) else {
                return;
            };
            let index = if let Some(delta) = pointer::wheel_delta(event) {
                let Some((offset, index)) = pointer::scroll_cursor(
                    state.scroll_offset,
                    active,
                    rows.len(),
                    visible,
                    delta,
                    |_| true,
                ) else {
                    return;
                };
                state.scroll_offset = offset;
                index
            } else if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                let Some((x, y)) = position else {
                    return;
                };
                let Some(row) = y.checked_sub(header as usize) else {
                    return;
                };
                let widths = self.compute_column_widths(&columns, content_width, state);
                let mut start = 0usize;
                let Some(column) = widths.iter().position(|width| {
                    let hit = x >= start && x < start + *width as usize;
                    start += *width as usize + 1;
                    hit
                }) else {
                    return;
                };
                let offset =
                    ensure_item_visible(state.scroll_offset.min(rows.len() - 1), active, visible);
                let index = offset + row;
                if row >= visible || index >= rows.len() {
                    return;
                }
                state.active_column = column;
                state.scroll_offset = offset;
                if self.rows[rows[index]].disabled {
                    return;
                }
                let id = &self.rows[rows[index]].id;
                if self.selection_mode.is_multiple() {
                    state.selection.toggle(id);
                } else {
                    state.selection.replace_selection(id.clone());
                }
                index
            } else {
                return;
            };
            let index = if self.rows[rows[index]].disabled {
                let Some(enabled) = rows
                    .iter()
                    .enumerate()
                    .skip(index)
                    .find(|(_, i)| !self.rows[**i].disabled)
                    .or_else(|| {
                        rows.iter()
                            .enumerate()
                            .take(index)
                            .rev()
                            .find(|(_, i)| !self.rows[**i].disabled)
                    })
                else {
                    return;
                };
                enabled.0
            } else {
                index
            };
            let id = self.rows[rows[index]].id.clone();
            set_active_row(state, id.clone());
            state.active_column = state.active_column.min(columns.len() - 1);
            (id, columns[state.active_column].1.id.clone())
        };
        ctx.set_handled();
        if let Some(handler) = &self.on_change {
            ctx.emit(handler(id.clone()));
        }
        if self.navigation_mode == DataTableNavigationMode::Cell {
            if let Some(handler) = &self.on_cell_change {
                ctx.emit(handler(id, column_id));
            }
        }
    }

    fn has_enabled_rows(&self) -> bool {
        self.visible_row_indices()
            .iter()
            .any(|i| !self.rows[*i].disabled)
    }

    fn active_index_from_state(
        &self,
        state: &DataTableState,
        visible_indices: &[usize],
    ) -> Option<usize> {
        active_row_id(state)
            .as_deref()
            .and_then(|id| {
                visible_indices
                    .iter()
                    .position(|index| self.rows[*index].id == id && !self.rows[*index].disabled)
            })
            .or_else(|| {
                (self.active_row.is_none()
                    && state.active_row.is_none()
                    && !state.active_row_initialized)
                    .then(|| visible_indices.iter().position(|i| !self.rows[*i].disabled))
                    .flatten()
            })
    }

    fn navigation_index(&self, state: &DataTableState, rows: &[usize]) -> Option<usize> {
        self.active_index_from_state(state, rows)
            .or_else(|| rows.iter().position(|i| !self.rows[*i].disabled))
    }

    fn pad_cell(
        text: &str,
        width: usize,
        align: TableAlign,
        metrics: crate::render::text::TextMetrics,
    ) -> String {
        if width == 0 {
            return String::new();
        }

        let truncated = metrics.clip(text, 0, width);
        let text_width = metrics.width(&truncated);
        let remaining = width.saturating_sub(text_width);

        match align {
            TableAlign::Left => format!("{truncated}{}", " ".repeat(remaining)),
            TableAlign::Right => format!("{}{truncated}", " ".repeat(remaining)),
            TableAlign::Center => {
                let left = remaining / 2;
                let right = remaining.saturating_sub(left);
                format!("{}{}{}", " ".repeat(left), truncated, " ".repeat(right))
            }
        }
    }

    fn visible_columns(&self) -> Vec<(usize, &DataTableColumn)> {
        self.columns
            .iter()
            .enumerate()
            .filter(|(_, column)| {
                column.visible
                    && !self
                        .hidden_columns
                        .iter()
                        .any(|hidden| hidden == &column.id)
            })
            .collect()
    }

    fn column_width_override<'a>(
        state: &'a DataTableState,
        column_id: &str,
    ) -> Option<&'a (String, u16)> {
        state
            .column_widths
            .iter()
            .find(|(id, _)| id.as_str() == column_id)
    }

    fn set_column_width(state: &mut DataTableState, column_id: &str, width: u16) {
        if let Some((_, stored_width)) = state
            .column_widths
            .iter_mut()
            .find(|(id, _)| id.as_str() == column_id)
        {
            *stored_width = width.max(3);
        } else {
            state
                .column_widths
                .push((column_id.to_owned(), width.max(3)));
        }
    }

    fn compute_column_widths(
        &self,
        visible_columns: &[(usize, &DataTableColumn)],
        total_width: u16,
        state: &DataTableState,
    ) -> Vec<u16> {
        if visible_columns.is_empty() || total_width == 0 {
            return Vec::new();
        }

        let separator_count = visible_columns.len().saturating_sub(1) as u16;
        if total_width <= separator_count {
            return vec![1; visible_columns.len()];
        }

        let available = total_width - separator_count;
        let mut widths: Vec<u16> = visible_columns
            .iter()
            .map(|(_, column)| {
                Self::column_width_override(state, &column.id)
                    .map(|(_, width)| *width)
                    .unwrap_or(column.width)
                    .max(3)
            })
            .collect();
        let mut used = widths
            .iter()
            .map(|width| usize::from(*width))
            .sum::<usize>();
        let available = usize::from(available);

        if used < available {
            if let Some(last) = widths.last_mut() {
                *last += (available - used) as u16;
            }
            return widths;
        }

        while used > available {
            let mut changed = false;
            for width in widths.iter_mut().rev() {
                if *width > 3 && used > available {
                    *width -= 1;
                    used -= 1;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        widths
    }

    fn draw_row(
        chunk: &mut crate::render::chunk::Chunk,
        x: u16,
        y: u16,
        widths: &[u16],
        visible_columns: &[(usize, &DataTableColumn)],
        cells: &[String],
        style: crate::render::style::Style,
        active_cell: Option<(usize, crate::render::style::Style)>,
    ) {
        let mut cursor_x = x;

        for (index, width) in widths.iter().enumerate() {
            if *width == 0 {
                continue;
            }

            let (source_index, column) = visible_columns[index];
            let cell_text = cells.get(source_index).map(String::as_str).unwrap_or("");
            let rendered = Self::pad_cell(
                cell_text,
                *width as usize,
                column.align,
                chunk.text_metrics(),
            );
            let cell_style = active_cell
                .filter(|(active_index, _)| *active_index == index)
                .map(|(_, style)| style)
                .unwrap_or(style);
            let _ = chunk.fill(cursor_x, y, *width, 1, ' ', cell_style);
            let _ = chunk.set_string(cursor_x, y, &rendered, cell_style);
            cursor_x = cursor_x.saturating_add(*width);

            if index + 1 < widths.len() {
                let _ = chunk.set_char(cursor_x, y, '|', style);
                cursor_x = cursor_x.saturating_add(1);
            }
        }
    }

    fn header_cells(&self, visible_columns: &[(usize, &DataTableColumn)]) -> Vec<String> {
        visible_columns
            .iter()
            .map(|(_, column)| {
                if let Some(sort) = self.sort.as_ref() {
                    if sort.column_id == column.id {
                        let marker = match sort.direction {
                            DataTableSortDirection::Asc => " ^",
                            DataTableSortDirection::Desc => " v",
                        };
                        return format!("{}{}", column.title, marker);
                    }
                }
                column.title.clone()
            })
            .collect()
    }

    fn visible_row_indices(&self) -> Vec<usize> {
        let normalized_query = self
            .filter_query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .map(str::to_lowercase);

        let filterable_columns = self
            .columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| column.filterable.then_some(index))
            .collect::<Vec<_>>();
        let use_all_columns = filterable_columns.is_empty();

        let mut indices = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                let Some(query) = normalized_query.as_deref() else {
                    return true;
                };

                let cells = if use_all_columns {
                    row.cells.iter().enumerate().collect::<Vec<_>>()
                } else {
                    filterable_columns
                        .iter()
                        .filter_map(|column_index| {
                            row.cells
                                .get(*column_index)
                                .map(|cell| (*column_index, cell))
                        })
                        .collect::<Vec<_>>()
                };

                cells
                    .into_iter()
                    .any(|(_, cell)| cell.to_lowercase().contains(query))
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if let Some(sort) = self.sort.as_ref() {
            if let Some(column_index) = self
                .columns
                .iter()
                .position(|column| column.id == sort.column_id)
            {
                indices.sort_by(|left, right| {
                    let left_value = self.rows[*left]
                        .cells
                        .get(column_index)
                        .map(|cell| cell.to_lowercase())
                        .unwrap_or_default();
                    let right_value = self.rows[*right]
                        .cells
                        .get(column_index)
                        .map(|cell| cell.to_lowercase())
                        .unwrap_or_default();

                    match sort.direction {
                        DataTableSortDirection::Asc => left_value.cmp(&right_value),
                        DataTableSortDirection::Desc => right_value.cmp(&left_value),
                    }
                });
            }
        }

        indices
    }
}

impl<M> Default for DataTable<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for DataTable<M> {
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
        let header_style = theme.styles.surface_header.to_render_style();
        let muted_style = theme.styles.text_muted.to_render_style();
        let disabled_style = theme.styles.interactive_disabled.to_render_style();
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

        let state = ctx.state_or_default::<DataTableState>();
        let visible_columns = self.visible_columns();

        if visible_columns.is_empty() {
            let message = metrics.clip("Add columns to render a table", 0, content_width as usize);
            let _ = chunk.set_string(content_x, content_y, &message, muted_style);
            return;
        }

        let widths = self.compute_column_widths(&visible_columns, content_width, state);
        if widths.is_empty() {
            return;
        }

        let mut rows_y = content_y;
        let mut body_height = content_height;

        if self.sticky_header {
            let header_cells = self.header_cells(&visible_columns);
            Self::draw_row(
                chunk,
                content_x,
                content_y,
                &widths,
                &visible_columns,
                &header_cells,
                header_style,
                None,
            );

            if content_height <= 1 {
                return;
            }

            let separator_y = content_y + 1;
            let _ = chunk.fill(content_x, separator_y, content_width, 1, '-', muted_style);
            let mut separator_x = content_x;
            for (index, width) in widths.iter().enumerate() {
                separator_x = separator_x.saturating_add(*width);
                if index + 1 < widths.len() {
                    let _ = chunk.set_char(separator_x, separator_y, '+', muted_style);
                    separator_x = separator_x.saturating_add(1);
                }
            }

            if content_height <= 2 {
                return;
            }
            rows_y = content_y + 2;
            body_height = content_height.saturating_sub(2);
        }

        let visible_indices = self.visible_row_indices();
        if visible_indices.is_empty() {
            let message = metrics.clip(&self.empty_message, 0, content_width as usize);
            let _ = chunk.set_string(content_x, rows_y, &message, muted_style);
            return;
        }

        let active_index = self.active_index_from_state(state, &visible_indices);
        let visible_rows = body_height as usize;
        let mut scroll_offset = state
            .scroll_offset
            .min(visible_indices.len().saturating_sub(1));
        if let Some(active_index) = active_index {
            scroll_offset = ensure_item_visible(scroll_offset, active_index, visible_rows);
        }

        for row in 0..visible_rows {
            let visible_row_index = scroll_offset + row;
            if visible_row_index >= visible_indices.len() {
                break;
            }

            let row_index = visible_indices[visible_row_index];
            let row_item = &self.rows[row_index];
            let is_active = active_index == Some(visible_row_index);
            let is_selected = state.selection.is_selected(&row_item.id);
            let style = if row_item.disabled {
                disabled_style
            } else if is_active && self.navigation_mode == DataTableNavigationMode::Row {
                active_style
            } else if is_selected {
                theme.styles.selected.to_render_style()
            } else {
                row_style
            };
            let y = rows_y + row as u16;
            let _ = chunk.fill(content_x, y, content_width, 1, ' ', style);
            let active_cell = if is_active && self.navigation_mode == DataTableNavigationMode::Cell
            {
                Some((
                    state
                        .active_column
                        .min(visible_columns.len().saturating_sub(1)),
                    active_style,
                ))
            } else {
                None
            };
            Self::draw_row(
                chunk,
                content_x,
                y,
                &widths,
                &visible_columns,
                &row_item.cells,
                style,
                active_cell,
            );
        }
    }

    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() == Lifecycle::Unmount {
            return;
        }
        let mount = ctx.lifecycle() == Lifecycle::Mount;
        let state = ctx.state_mut::<DataTableState>();
        if mount {
            if let Some(value) = &self.default_active_row {
                state.active_row = value.clone();
                state.active_row_initialized = true;
            }
            state
                .selection
                .set_selected_ids(self.default_selected_ids.clone());
        }
        if let Some(value) = &self.active_row {
            state.active_row = value.clone();
            state.active_row_initialized = true;
        } else if let Some(value) = state.controlled_active.take() {
            state.active_row = value;
            state.active_row_initialized = true;
        } else if state
            .active_row
            .as_ref()
            .is_some_and(|id| !self.rows.iter().any(|r| &r.id == id))
        {
            state.active_row = self
                .visible_row_indices()
                .iter()
                .find(|i| !self.rows[**i].disabled)
                .map(|i| self.rows[*i].id.clone());
        }
        if let Some(ids) = &self.selected_ids {
            state.selection.set_selected_ids(ids.clone());
        } else if let Some(ids) = state.controlled_selection.take() {
            state.selection.set_selected_ids(ids);
        } else {
            state
                .selection
                .selected
                .retain(|id| self.rows.iter().any(|r| &r.id == id));
        }
        if self.selection_mode == SelectionMode::Single {
            state.selection.selected.truncate(1);
        }
        state.selection.cursor = state.active_row.clone();
        state.controlled_active = self.active_row.clone();
        state.controlled_selection = self
            .selected_ids
            .as_ref()
            .map(|_| state.selection.selected.clone());
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target {
            return;
        }
        let before = ctx.state_mut::<DataTableState>().clone();
        self.handle_input(event, ctx);
        let after = ctx.state_mut::<DataTableState>().clone();
        if before.active_row != after.active_row {
            if let Some(handler) = &self.on_active_row_change {
                ctx.emit(handler(after.active_row));
            }
        }
        if before.selection.selected != after.selection.selected {
            if let Some(handler) = &self.on_selection_change {
                ctx.emit(handler(after.selection.selected));
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
        if self.disabled || !self.has_enabled_rows() {
            FocusConfig::None
        } else {
            FocusConfig::Composite
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a new data table widget.
pub fn data_table<M>() -> DataTable<M> {
    DataTable::new()
}

fn active_row_id(state: &DataTableState) -> Option<String> {
    state.active_row.clone()
}

fn set_active_row(state: &mut DataTableState, id: String) {
    state.active_row_initialized = true;
    state.active_row = Some(id.clone());
    state.selection.set_cursor(Some(id));
}

fn sync_state_aliases(state: &mut DataTableState) {
    state.selection.cursor = state.active_row.clone();
}

impl<M> DataTable<M> {
    fn preferred_width(&self) -> u16 {
        self.preferred_width_with(crate::render::text::TextMetrics::default())
    }

    fn preferred_width_with(&self, metrics: crate::render::text::TextMetrics) -> u16 {
        let visible = self.visible_columns();
        let separator_width = visible.len().saturating_sub(1) as u16;
        let content_width = visible
            .iter()
            .map(|(_, column)| {
                column
                    .width
                    .max(metrics.width(&column.title).min(u16::MAX as usize) as u16)
            })
            .sum::<u16>()
            .saturating_add(separator_width);
        content_width
            .max(16)
            .saturating_add(if self.border.is_some() { 2 } else { 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DataTable, DataTableColumn, DataTableRow, DataTableSort, DataTableSortDirection,
        DataTableState,
    };

    #[test]
    fn filter_query_uses_filterable_columns() {
        let table = DataTable::<()>::new()
            .columns([
                DataTableColumn::new("Service")
                    .id("service")
                    .filterable(true),
                DataTableColumn::new("Owner").id("owner"),
            ])
            .rows([
                DataTableRow::new("api", ["api", "platform"]),
                DataTableRow::new("search", ["search", "relevance"]),
            ])
            .filter_query("sea");

        assert_eq!(table.visible_row_indices(), vec![1]);
    }

    #[test]
    fn sorting_reorders_visible_rows() {
        let table = DataTable::<()>::new()
            .columns([
                DataTableColumn::new("Service").id("service").sortable(true),
                DataTableColumn::new("Stage").id("stage"),
            ])
            .rows([
                DataTableRow::new("search", ["search", "prod"]),
                DataTableRow::new("api", ["api", "prod"]),
            ])
            .sort(DataTableSort::new("service", DataTableSortDirection::Asc));

        assert_eq!(table.visible_row_indices(), vec![1, 0]);
    }

    #[test]
    fn hidden_columns_are_removed_from_render_columns() {
        let table = DataTable::<()>::new()
            .columns([
                DataTableColumn::new("Service").id("service"),
                DataTableColumn::new("Owner").id("owner"),
            ])
            .hidden_column("owner");

        let columns = table.visible_columns();

        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].1.id, "service");
    }

    #[test]
    fn state_column_width_overrides_definition() {
        let table = DataTable::<()>::new().columns([
            DataTableColumn::new("Service").id("service").width(10),
            DataTableColumn::new("Owner").id("owner").width(8),
        ]);
        let columns = table.visible_columns();
        let mut state = DataTableState::default();

        DataTable::<()>::set_column_width(&mut state, "service", 16);
        let widths = table.compute_column_widths(&columns, 30, &state);

        assert_eq!(widths[0], 16);
    }
}
