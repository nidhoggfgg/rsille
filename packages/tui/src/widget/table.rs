//! Table widget - interactive tabular data display component
//!
//! A modern table component with support for:
//! - Flexible column definitions with configurable widths
//! - Row selection (single, multiple, or none)
//! - Keyboard navigation (arrows, PageUp/Down, Home/End)
//! - Mouse interaction and scroll
//! - Scrollable viewport for large datasets
//! - Custom cell rendering
//! - Empty state message
//! - Custom styling

use super::*;
use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::style::{BorderStyle, Style, ThemeManager};
use std::sync::Arc;

/// Table visual variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableVariant {
    /// Default table with simple dividers
    #[default]
    Simple,
    /// Table with full borders
    Bordered,
    /// Compact table without horizontal dividers
    Compact,
    /// Striped rows (alternating background)
    Striped,
}

/// Column width strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnWidth {
    /// Fixed width in characters
    Fixed(u16),
    /// Flexible width with flex factor (proportional distribution)
    Flex(u16),
    /// Auto-sized to content
    Auto,
}

/// Column definition
#[derive(Clone)]
pub struct Column<T: Clone> {
    /// Column header title
    pub title: String,
    /// Column width strategy
    pub width: ColumnWidth,
    /// Cell value accessor function
    pub accessor: Arc<dyn Fn(&T) -> String + Send + Sync>,
}

impl<T: Clone> Column<T> {
    /// Create a new column with a title and accessor function
    pub fn new<F>(title: impl Into<String>, accessor: F) -> Self
    where
        F: Fn(&T) -> String + Send + Sync + 'static,
    {
        Self {
            title: title.into(),
            width: ColumnWidth::Auto,
            accessor: Arc::new(accessor),
        }
    }

    /// Set the column width strategy
    pub fn width(mut self, width: ColumnWidth) -> Self {
        self.width = width;
        self
    }
}

impl<T: Clone> std::fmt::Debug for Column<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Column")
            .field("title", &self.title)
            .field("width", &self.width)
            .finish()
    }
}

/// Selection event information
#[derive(Debug, Clone)]
pub struct TableSelectionEvent<T: Clone> {
    /// The selected row data
    pub selected_rows: Vec<T>,
    /// Current focused row index
    pub focused_index: Option<usize>,
    /// Current scroll offset
    pub scroll_offset: usize,
}

/// Interactive table widget
///
/// A modern table component with support for:
/// - Flexible column definitions
/// - Single and multiple selection modes
/// - Keyboard navigation
/// - Mouse interaction
/// - Scrollable viewport
/// - Custom styling
///
/// # Examples
/// ```
/// use tui::widget::{Table, Column, SelectionMode, ColumnWidth};
///
/// #[derive(Clone, Debug)]
/// struct User {
///     name: String,
///     email: String,
///     age: u32,
/// }
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     UserSelected(Vec<User>),
/// }
///
/// let columns = vec![
///     Column::new("Name", |user: &User| user.name.clone())
///         .width(ColumnWidth::Flex(2)),
///     Column::new("Email", |user: &User| user.email.clone())
///         .width(ColumnWidth::Flex(3)),
///     Column::new("Age", |user: &User| user.age.to_string())
///         .width(ColumnWidth::Fixed(5)),
/// ];
///
/// let users = vec![
///     User { name: "Alice".into(), email: "alice@example.com".into(), age: 30 },
///     User { name: "Bob".into(), email: "bob@example.com".into(), age: 25 },
/// ];
///
/// let table = Table::new(columns)
///     .rows(users)
///     .selection_mode(SelectionMode::Single)
///     .on_select(|event| Message::UserSelected(event.selected_rows));
/// ```
#[derive(Clone)]
pub struct Table<T: Clone, M = ()> {
    columns: Vec<Column<T>>,
    rows: Vec<T>,
    selected_indices: Vec<usize>,
    focused_index: Option<usize>,
    selection_mode: SelectionMode,
    scroll_offset: usize,
    horizontal_scroll: u16,
    viewport_height: u16,
    empty_message: String,
    show_scrollbar: bool,
    show_header: bool,
    variant: TableVariant,
    border: Option<BorderStyle>,
    focused: bool,
    custom_style: Option<Style>,
    custom_header_style: Option<Style>,
    custom_focus_style: Option<Style>,
    custom_selected_style: Option<Style>,
    custom_striped_style: Option<Style>,
    on_select: Option<Arc<dyn Fn(TableSelectionEvent<T>) -> M + Send + Sync>>,
}

impl<T: Clone, M> std::fmt::Debug for Table<T, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Table")
            .field("columns", &self.columns.len())
            .field("rows", &self.rows.len())
            .field("selected_indices", &self.selected_indices)
            .field("focused_index", &self.focused_index)
            .field("selection_mode", &self.selection_mode)
            .field("scroll_offset", &self.scroll_offset)
            .field("horizontal_scroll", &self.horizontal_scroll)
            .field("viewport_height", &self.viewport_height)
            .field("show_header", &self.show_header)
            .field("focused", &self.focused)
            .finish()
    }
}

impl<T: Clone, M> Table<T, M> {
    /// Create a new table with column definitions
    pub fn new(columns: Vec<Column<T>>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            selected_indices: Vec::new(),
            focused_index: None,
            selection_mode: SelectionMode::default(),
            scroll_offset: 0,
            horizontal_scroll: 0,
            viewport_height: 10,
            empty_message: "No data".to_string(),
            show_scrollbar: true,
            show_header: true,
            variant: TableVariant::default(),
            border: None,
            focused: false,
            custom_style: None,
            custom_header_style: None,
            custom_focus_style: None,
            custom_selected_style: None,
            custom_striped_style: None,
            on_select: None,
        }
    }

    /// Create a static table (display-only, no selection)
    ///
    /// This is a convenience method that creates a table with SelectionMode::None
    /// and no scrollbar, optimized for static data display.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{Table, Column};
    ///
    /// #[derive(Clone)]
    /// struct Item { name: String, value: String }
    ///
    /// let columns = vec![
    ///     Column::new("Name", |item: &Item| item.name.clone()),
    ///     Column::new("Value", |item: &Item| item.value.clone()),
    /// ];
    ///
    /// let table = Table::<Item, ()>::static_table(columns)
    ///     .rows(vec![
    ///         Item { name: "Key1".into(), value: "Value1".into() },
    ///         Item { name: "Key2".into(), value: "Value2".into() },
    ///     ]);
    /// ```
    pub fn static_table(columns: Vec<Column<T>>) -> Self {
        Self::new(columns)
            .selection_mode(SelectionMode::None)
            .show_scrollbar(false)
    }

    /// Set the table rows
    pub fn rows(mut self, rows: Vec<T>) -> Self {
        self.rows = rows;
        // Auto-focus first row if any rows exist
        if !self.rows.is_empty() && self.focused_index.is_none() {
            self.focused_index = Some(0);
        }
        self
    }

    /// Add a single row
    pub fn row(mut self, row: T) -> Self {
        self.rows.push(row);
        if self.focused_index.is_none() {
            self.focused_index = Some(0);
        }
        self
    }

    /// Set the selection mode
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Set the focused row index
    pub fn focused_index(mut self, index: Option<usize>) -> Self {
        if let Some(idx) = index {
            if idx < self.rows.len() {
                self.focused_index = Some(idx);
                self.ensure_focused_visible();
            }
        }
        self
    }

    /// Set the scroll offset
    pub fn scroll_offset(mut self, offset: usize) -> Self {
        self.scroll_offset = offset.min(self.rows.len().saturating_sub(1));
        self
    }

    /// Set the viewport height
    pub fn viewport_height(mut self, height: u16) -> Self {
        self.viewport_height = height;
        self
    }

    /// Set the empty state message
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = message.into();
        self
    }

    /// Set whether to show the scrollbar
    pub fn show_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }

    /// Set whether to show the header row
    pub fn show_header(mut self, show: bool) -> Self {
        self.show_header = show;
        self
    }

    /// Set initially selected row indices
    pub fn selected(mut self, indices: Vec<usize>) -> Self {
        self.selected_indices = indices
            .into_iter()
            .filter(|&idx| idx < self.rows.len())
            .collect();
        self
    }

    /// Set the selection change handler
    pub fn on_select<F>(mut self, handler: F) -> Self
    where
        F: Fn(TableSelectionEvent<T>) -> M + Send + Sync + 'static,
    {
        self.on_select = Some(Arc::new(handler));
        self
    }

    /// Set a custom style (overrides theme styling)
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Set a custom header style
    pub fn header_style(mut self, style: Style) -> Self {
        self.custom_header_style = Some(style);
        self
    }

    /// Set a custom focus style
    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Set a custom selected row style
    pub fn selected_style(mut self, style: Style) -> Self {
        self.custom_selected_style = Some(style);
        self
    }

    /// Set a custom striped row style (for alternating rows)
    pub fn striped_style(mut self, style: Style) -> Self {
        self.custom_striped_style = Some(style);
        self
    }

    /// Set the table visual variant
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{Table, Column, TableVariant};
    ///
    /// let table = Table::<String, ()>::new(vec![])
    ///     .variant(TableVariant::Bordered);
    /// ```
    pub fn variant(mut self, variant: TableVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the border style for bordered table variants
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let table = Table::<String, ()>::new(vec![])
    ///     .variant(TableVariant::Bordered)
    ///     .border(BorderStyle::Rounded);
    /// ```
    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    /// Compute column widths based on available space
    fn compute_column_widths(&self, available_width: u16) -> Vec<u16> {
        if self.columns.is_empty() {
            return Vec::new();
        }

        let mut widths = vec![0u16; self.columns.len()];
        if available_width == 0 {
            return widths;
        }

        let mut remaining_width = available_width;
        let mut flex_total = 0u16;
        let mut flex_indices = Vec::new();

        // First pass: assign fixed/auto widths and reserve separator space evenly
        for (i, col) in self.columns.iter().enumerate() {
            match col.width {
                ColumnWidth::Fixed(w) => {
                    let assigned = w.min(remaining_width);
                    widths[i] = assigned;
                    remaining_width = remaining_width.saturating_sub(assigned);
                }
                ColumnWidth::Auto => {
                    use unicode_width::UnicodeWidthStr;
                    let mut max_width = col.title.width() as u16;
                    for row in &self.rows {
                        let cell_value = (col.accessor)(row);
                        max_width = max_width.max(cell_value.width() as u16);
                    }
                    let assigned = max_width.min(remaining_width);
                    widths[i] = assigned;
                    remaining_width = remaining_width.saturating_sub(assigned);
                }
                ColumnWidth::Flex(flex) => {
                    flex_total = flex_total.saturating_add(flex);
                    flex_indices.push(i);
                }
            }

            // Reserve space for column dividers (except after the last column)
            if i < self.columns.len() - 1 {
                remaining_width = remaining_width.saturating_sub(1);
            }
        }

        // Second pass: distribute remaining content width to flex columns
        if flex_total > 0 && remaining_width > 0 {
            let mut distributed = 0u16;
            for (idx, &col_index) in flex_indices.iter().enumerate() {
                let flex = match self.columns[col_index].width {
                    ColumnWidth::Flex(f) => f,
                    _ => continue,
                };

                let is_last = idx == flex_indices.len().saturating_sub(1);
                let share = if is_last {
                    remaining_width.saturating_sub(distributed)
                } else if flex_total == 0 {
                    0
                } else {
                    ((remaining_width as u32 * flex as u32) / flex_total as u32) as u16
                };

                widths[col_index] = share;
                distributed = distributed.saturating_add(share);
            }
        }

        widths
    }

    /// Move focus to the next row
    fn focus_next(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let next = self.focused_index.map(|i| (i + 1).min(self.rows.len() - 1)).unwrap_or(0);
        self.focused_index = Some(next);
        self.ensure_focused_visible();
    }

    /// Move focus to the previous row
    fn focus_previous(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let prev = self.focused_index.map(|i| i.saturating_sub(1)).unwrap_or(0);
        self.focused_index = Some(prev);
        self.ensure_focused_visible();
    }

    /// Move focus to the first row
    fn focus_first(&mut self) {
        if !self.rows.is_empty() {
            self.focused_index = Some(0);
            self.ensure_focused_visible();
        }
    }

    /// Move focus to the last row
    fn focus_last(&mut self) {
        if !self.rows.is_empty() {
            self.focused_index = Some(self.rows.len() - 1);
            self.ensure_focused_visible();
        }
    }

    /// Jump focus forward by a page
    fn page_down(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let page_size = self.viewport_height.saturating_sub(1).max(1) as usize;
        let current = self.focused_index.unwrap_or(0);
        self.focused_index = Some((current + page_size).min(self.rows.len() - 1));
        self.ensure_focused_visible();
    }

    /// Jump focus backward by a page
    fn page_up(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let page_size = self.viewport_height.saturating_sub(1).max(1) as usize;
        let current = self.focused_index.unwrap_or(0);
        self.focused_index = Some(current.saturating_sub(page_size));
        self.ensure_focused_visible();
    }

    /// Ensure the focused row is visible in the viewport
    fn ensure_focused_visible(&mut self) {
        if let Some(focused_idx) = self.focused_index {
            let viewport_size = self.viewport_height as usize;

            // If focused row is above viewport, scroll up
            if focused_idx < self.scroll_offset {
                self.scroll_offset = focused_idx;
            }
            // If focused row is beyond viewport, scroll down
            else if focused_idx >= self.scroll_offset + viewport_size {
                self.scroll_offset = focused_idx.saturating_sub(viewport_size - 1);
            }
        }
    }

    /// Toggle selection of the focused row
    fn toggle_selection(&mut self) -> Vec<M> {
        if self.selection_mode == SelectionMode::None {
            return vec![];
        }

        let Some(focused_idx) = self.focused_index else {
            return vec![];
        };

        match self.selection_mode {
            SelectionMode::Single => {
                // Toggle: if already selected, deselect; otherwise select only this row
                if self.selected_indices.contains(&focused_idx) {
                    self.selected_indices.clear();
                } else {
                    self.selected_indices = vec![focused_idx];
                }
            }
            SelectionMode::Multiple => {
                // Toggle: add or remove from selection
                if let Some(pos) = self.selected_indices.iter().position(|&idx| idx == focused_idx) {
                    self.selected_indices.remove(pos);
                } else {
                    self.selected_indices.push(focused_idx);
                }
            }
            SelectionMode::None => {}
        }

        self.emit_selection()
    }

    /// Emit selection change event
    fn emit_selection(&self) -> Vec<M> {
        if let Some(ref handler) = self.on_select {
            let selected_rows: Vec<T> = self
                .selected_indices
                .iter()
                .filter_map(|&idx| self.rows.get(idx).cloned())
                .collect();

            let event = TableSelectionEvent {
                selected_rows,
                focused_index: self.focused_index,
                scroll_offset: self.scroll_offset,
            };

            let message = handler(event);
            vec![message]
        } else {
            vec![]
        }
    }

    /// Get pre-computed styles for all row states
    fn get_styles(&self) -> TableStyles {
        ThemeManager::global().with_theme(|theme| {
            let normal = self
                .custom_style
                .unwrap_or(Style::default().fg(theme.colors.text))
                .to_render_style();

            let header = self
                .custom_header_style
                .unwrap_or(Style::default()
                    .fg(theme.colors.text)
                    .bold())
                .to_render_style();

            let focused = self
                .custom_focus_style
                .unwrap_or(Style::default()
                    .fg(theme.colors.text)
                    .bg(theme.colors.focus_background))
                .to_render_style();

            let selected = self
                .custom_selected_style
                .unwrap_or(Style::default()
                    .fg(theme.colors.text)
                    .bg(theme.colors.primary))
                .to_render_style();

            // Striped style for alternating rows
            let striped = self
                .custom_striped_style
                .unwrap_or(Style::default()
                    .fg(theme.colors.text)
                    .bg(theme.colors.surface))
                .to_render_style();

            let border = Style::default()
                .fg(theme.colors.border)
                .bg(theme.colors.background)
                .to_render_style();

            // Column separator only has foreground color, allowing it to inherit row background
            let column_separator = Style::default()
                .fg(theme.colors.border)
                .to_render_style();

            // Header border uses same style as normal border
            let header_border = border;

            let scrollbar_track = Style::default().fg(theme.colors.border).to_render_style();
            let scrollbar_thumb = Style::default().fg(theme.colors.info).to_render_style();

            TableStyles {
                normal,
                header,
                header_border,
                focused,
                selected,
                striped,
                border,
                column_separator,
                scrollbar_track,
                scrollbar_thumb,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Item;

    #[test]
    fn flex_columns_reserve_separator_space() {
        let columns = vec![
            Column::new("ID", |_: &Item| "1".into()).width(ColumnWidth::Fixed(5)),
            Column::new("Product", |_: &Item| "Laptop".into()).width(ColumnWidth::Flex(3)),
            Column::new("Category", |_: &Item| "Electronics".into()).width(ColumnWidth::Flex(2)),
            Column::new("Price", |_: &Item| "$999.99".into()).width(ColumnWidth::Fixed(10)),
            Column::new("Stock", |_: &Item| "15".into()).width(ColumnWidth::Fixed(7)),
        ];

        let table = Table::<Item, ()>::new(columns);
        let available_width = 40;
        let widths = table.compute_column_widths(available_width);
        let total_used = widths.iter().sum::<u16>() + widths.len().saturating_sub(1) as u16;

        assert!(
            total_used <= available_width,
            "computed widths {widths:?} exceed available width {available_width}"
        );
    }
}

/// Pre-computed styles for efficient rendering
struct TableStyles {
    normal: render::style::Style,
    header: render::style::Style,
    header_border: render::style::Style,
    focused: render::style::Style,
    selected: render::style::Style,
    striped: render::style::Style,
    border: render::style::Style,
    column_separator: render::style::Style,
    scrollbar_track: render::style::Style,
    scrollbar_thumb: render::style::Style,
}

impl<T: Clone + Send + Sync, M: Send + Sync> Widget<M> for Table<T, M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let width = area.width();
        let height = area.height();

        // Handle empty state
        if self.rows.is_empty() || self.columns.is_empty() {
            let styles = self.get_styles();
            use unicode_width::UnicodeWidthStr;
            let msg_width = self.empty_message.width() as u16;
            let x = if width > msg_width { (width - msg_width) / 2 } else { 0 };
            let y = height / 2;
            let _ = chunk.set_string(x, y, &self.empty_message, styles.normal);
            return;
        }

        let styles = self.get_styles();

        // For bordered variant, render outer border
        let (content_x, content_y, content_width, content_height) = if self.variant == TableVariant::Bordered {
            self.render_border(chunk, width, height, &styles);
            // Reserve space for borders
            (1, 1, width.saturating_sub(2), height.saturating_sub(2))
        } else {
            (0, 0, width, height)
        };

        // Calculate column widths
        let scrollbar_width = if self.show_scrollbar && self.rows.len() > self.viewport_height as usize {
            2
        } else {
            0
        };
        let computed_widths = self.compute_column_widths(content_width.saturating_sub(scrollbar_width));

        let mut y = content_y;

        // Render header
        if self.show_header && y < content_y + content_height {
            self.render_header(chunk, content_x, y, content_width, scrollbar_width, &computed_widths, &styles);
            y += 1;

            // Render header separator (except for Compact variant)
            if self.variant != TableVariant::Compact && y < content_y + content_height {
                self.render_header_separator(chunk, content_x, y, content_width, scrollbar_width, &computed_widths, &styles);
                y += 1;
            }
        }

        // Render rows
        let viewport_rows = (content_y + content_height).saturating_sub(y);
        let visible_start = self.scroll_offset;
        let visible_end = (visible_start + viewport_rows as usize).min(self.rows.len());

        for row_idx in visible_start..visible_end {
            if y >= content_y + content_height {
                break;
            }

            self.render_row(
                chunk,
                content_x,
                y,
                content_width,
                scrollbar_width,
                row_idx,
                &computed_widths,
                &styles,
            );

            y += 1;
        }

        // Render scrollbar if needed
        if self.show_scrollbar && self.rows.len() > self.viewport_height as usize {
            let scrollbar_x = if self.variant == TableVariant::Bordered {
                width.saturating_sub(3) // Account for border
            } else {
                width.saturating_sub(2)
            };
            let scrollbar_y_start = if self.show_header {
                content_y + if self.variant != TableVariant::Compact { 2 } else { 1 }
            } else {
                content_y
            };
            self.render_scrollbar(
                chunk,
                scrollbar_x,
                scrollbar_y_start,
                content_y + content_height,
                &styles,
            );
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Down => {
                    self.focus_next();
                    EventResult::Consumed(vec![])
                }
                KeyCode::Up => {
                    self.focus_previous();
                    EventResult::Consumed(vec![])
                }
                KeyCode::Home => {
                    self.focus_first();
                    EventResult::Consumed(vec![])
                }
                KeyCode::End => {
                    self.focus_last();
                    EventResult::Consumed(vec![])
                }
                KeyCode::PageDown => {
                    self.page_down();
                    EventResult::Consumed(vec![])
                }
                KeyCode::PageUp => {
                    self.page_up();
                    EventResult::Consumed(vec![])
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let messages = self.toggle_selection();
                    EventResult::Consumed(messages)
                }
                _ => EventResult::Ignored,
            },
            Event::Mouse(mouse_event) => match mouse_event.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let messages = self.toggle_selection();
                    EventResult::Consumed(messages)
                }
                MouseEventKind::ScrollDown => {
                    self.focus_next();
                    EventResult::Consumed(vec![])
                }
                MouseEventKind::ScrollUp => {
                    self.focus_previous();
                    EventResult::Consumed(vec![])
                }
                _ => EventResult::Ignored,
            },
            _ => EventResult::Ignored,
        }
    }

    fn constraints(&self) -> Constraints {
        use unicode_width::UnicodeWidthStr;

        if self.columns.is_empty() {
            return Constraints {
                min_width: 20,
                max_width: Some(20),
                min_height: 3,
                max_height: Some(3),
                flex: None,
            };
        }

        // Calculate total width needed
        let mut total_width = 0u16;
        for col in &self.columns {
            match col.width {
                ColumnWidth::Fixed(w) => total_width += w,
                ColumnWidth::Auto => {
                    let mut max_width = col.title.width() as u16;
                    for row in &self.rows {
                        let cell_value = (col.accessor)(row);
                        max_width = max_width.max(cell_value.width() as u16);
                    }
                    total_width += max_width;
                }
                ColumnWidth::Flex(flex) => total_width += flex * 10, // Estimate
            }
            total_width += 1; // Separator
        }

        let scrollbar_width = if self.show_scrollbar && self.rows.len() > self.viewport_height as usize {
            2
        } else {
            0
        };

        total_width += scrollbar_width;

        // Calculate height
        let header_height = if self.show_header { 2 } else { 0 }; // Header + separator
        let content_height = if self.rows.is_empty() {
            1
        } else {
            self.rows.len().min(self.viewport_height as usize)
        };
        let total_height = (header_height + content_height) as u16;

        Constraints {
            min_width: total_width.max(20),
            max_width: Some(total_width.max(20)),
            min_height: total_height.max(3),
            max_height: Some(total_height.max(3)),
            flex: None,
        }
    }

    fn focusable(&self) -> bool {
        // Static tables (SelectionMode::None) should not be focusable
        self.selection_mode != SelectionMode::None && !self.rows.is_empty()
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if focused && self.focused_index.is_none() && !self.rows.is_empty() {
            self.focus_first();
        }
    }
}

impl<T: Clone, M> Table<T, M> {
    /// Render outer border for bordered variant
    fn render_border(
        &self,
        chunk: &mut render::chunk::Chunk,
        width: u16,
        height: u16,
        styles: &TableStyles,
    ) {
        let border_chars = self.border.unwrap_or(BorderStyle::Single).chars();

        // Top border
        let _ = chunk.set_char(0, 0, border_chars.top_left, styles.border);
        for x in 1..width.saturating_sub(1) {
            let _ = chunk.set_char(x, 0, border_chars.horizontal, styles.border);
        }
        let _ = chunk.set_char(width.saturating_sub(1), 0, border_chars.top_right, styles.border);

        // Side borders
        for y in 1..height.saturating_sub(1) {
            let _ = chunk.set_char(0, y, border_chars.vertical, styles.border);
            let _ = chunk.set_char(width.saturating_sub(1), y, border_chars.vertical, styles.border);
        }

        // Bottom border
        let _ = chunk.set_char(0, height.saturating_sub(1), border_chars.bottom_left, styles.border);
        for x in 1..width.saturating_sub(1) {
            let _ = chunk.set_char(x, height.saturating_sub(1), border_chars.horizontal, styles.border);
        }
        let _ = chunk.set_char(
            width.saturating_sub(1),
            height.saturating_sub(1),
            border_chars.bottom_right,
            styles.border,
        );
    }

    /// Render table header row
    fn render_header(
        &self,
        chunk: &mut render::chunk::Chunk,
        x_offset: u16,
        y: u16,
        width: u16,
        scrollbar_width: u16,
        computed_widths: &[u16],
        styles: &TableStyles,
    ) {
        let header_width = width.saturating_sub(scrollbar_width);
        let mut x = x_offset;
        for (i, col) in self.columns.iter().enumerate() {
            let col_width = computed_widths.get(i).copied().unwrap_or(10);

            // Check if we have enough space for this column (and separator if not last)
            let is_last_col = i == self.columns.len() - 1;
            let needed_width = if is_last_col { col_width } else { col_width + 1 };
            if x + needed_width > x_offset + header_width {
                break;
            }

            // Render header cell text
            use unicode_width::UnicodeWidthStr;
            let title = if col.title.width() as u16 > col_width {
                let mut truncated = String::new();
                let mut w = 0u16;
                for ch in col.title.chars() {
                    let ch_w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                    if w + ch_w + 1 > col_width {
                        truncated.push('…');
                        break;
                    }
                    truncated.push(ch);
                    w += ch_w;
                }
                truncated
            } else {
                col.title.clone()
            };

            let _ = chunk.set_string(x, y, &title, styles.header);

            x += col_width;
            if !is_last_col {
                // Use header_border style to maintain header background
                let _ = chunk.set_char(x, y, '│', styles.header_border);
                x += 1;
            }
        }
    }

    /// Render header separator line
    fn render_header_separator(
        &self,
        chunk: &mut render::chunk::Chunk,
        x_offset: u16,
        y: u16,
        width: u16,
        scrollbar_width: u16,
        computed_widths: &[u16],
        styles: &TableStyles,
    ) {
        if self.variant == TableVariant::Bordered {
            // For bordered tables, use junctions for cleaner look
            let border_chars = self.border.unwrap_or(BorderStyle::Single).chars();
            let mut x = x_offset;

            for (i, _) in self.columns.iter().enumerate() {
                let col_width = computed_widths.get(i).copied().unwrap_or(10);
                if x + col_width > x_offset + width.saturating_sub(scrollbar_width) {
                    break;
                }

                for _ in 0..col_width {
                    if x < x_offset + width.saturating_sub(scrollbar_width) {
                        let _ = chunk.set_char(x, y, border_chars.horizontal, styles.border);
                        x += 1;
                    }
                }

                if i < self.columns.len() - 1 && x < x_offset + width.saturating_sub(scrollbar_width) {
                    let _ = chunk.set_char(x, y, border_chars.cross, styles.border);
                    x += 1;
                }
            }
        } else {
            // Simple horizontal line for other variants
            for x in x_offset..(x_offset + width.saturating_sub(scrollbar_width)) {
                let _ = chunk.set_char(x, y, '─', styles.border);
            }
        }
    }

    /// Render a single table row
    fn render_row(
        &self,
        chunk: &mut render::chunk::Chunk,
        x_offset: u16,
        y: u16,
        width: u16,
        scrollbar_width: u16,
        row_idx: usize,
        computed_widths: &[u16],
        styles: &TableStyles,
    ) {
        let row = &self.rows[row_idx];
        let is_focused = Some(row_idx) == self.focused_index;
        let is_selected = self.selected_indices.contains(&row_idx);
        let is_striped = self.variant == TableVariant::Striped && row_idx % 2 == 1;

        let row_style = if is_selected {
            styles.selected
        } else if is_focused && self.focused {
            styles.focused
        } else if is_striped {
            styles.striped
        } else {
            styles.normal
        };

        // Fill row background
        if is_focused || is_selected || is_striped {
            let _ = chunk.fill(
                x_offset,
                y,
                width.saturating_sub(scrollbar_width),
                1,
                ' ',
                row_style,
            );
        }

        // Render cells
        let mut x = x_offset;
        for (i, col) in self.columns.iter().enumerate() {
            let col_width = computed_widths.get(i).copied().unwrap_or(10);

            // Check if we have enough space for this column (and separator if not last)
            let is_last_col = i == self.columns.len() - 1;
            let needed_width = if is_last_col { col_width } else { col_width + 1 };
            if x + needed_width > x_offset + width.saturating_sub(scrollbar_width) {
                break;
            }

            let cell_value = (col.accessor)(row);

            // Truncate cell value if needed
            use unicode_width::UnicodeWidthStr;
            let cell_text = if cell_value.width() as u16 > col_width {
                let mut truncated = String::new();
                let mut w = 0u16;
                for ch in cell_value.chars() {
                    let ch_w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                    if w + ch_w + 1 > col_width {
                        truncated.push('…');
                        break;
                    }
                    truncated.push(ch);
                    w += ch_w;
                }
                truncated
            } else {
                cell_value
            };

            let _ = chunk.set_string(x, y, &cell_text, row_style);

            x += col_width;
            if !is_last_col {
                let _ = chunk.set_char(x, y, '│', styles.column_separator);
                x += 1;
            }
        }

        // For Bordered variant, ensure the right border is not covered by background fill
        if self.variant == TableVariant::Bordered {
            let border_chars = self.border.unwrap_or(BorderStyle::Single).chars();
            let right_border_x = x_offset + width;
            let _ = chunk.set_char(right_border_x, y, border_chars.vertical, styles.border);
        }
    }

    /// Render scrollbar in the rightmost column
    fn render_scrollbar(
        &self,
        chunk: &mut render::chunk::Chunk,
        x_offset: u16,
        y_start: u16,
        y_end: u16,
        styles: &TableStyles,
    ) {
        let scrollbar_height = y_end.saturating_sub(y_start) as usize;
        let total_rows = self.rows.len();
        let viewport_size = self.viewport_height as usize;

        if scrollbar_height == 0 || total_rows <= viewport_size {
            return;
        }

        // Calculate scrollbar thumb position and size
        let thumb_size = ((viewport_size as f64 / total_rows as f64) * scrollbar_height as f64)
            .max(1.0)
            .round() as usize;

        let scroll_ratio = self.scroll_offset as f64 / (total_rows - viewport_size).max(1) as f64;
        let thumb_position = (scroll_ratio * (scrollbar_height - thumb_size) as f64).round() as usize;

        // Draw scrollbar track
        for y in y_start..y_end {
            let _ = chunk.set_char(x_offset, y, '│', styles.scrollbar_track);
        }

        // Draw scrollbar thumb
        for offset in 0..thumb_size {
            let y = y_start + (thumb_position + offset).min(scrollbar_height - 1) as u16;
            if y < y_end {
                let _ = chunk.set_char(x_offset, y, '█', styles.scrollbar_thumb);
            }
        }
    }
}

/// Create a new table widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// struct User { name: String, email: String }
///
/// let columns = vec![
///     Column::new("Name", |u: &User| u.name.clone()),
///     Column::new("Email", |u: &User| u.email.clone()),
/// ];
///
/// let table = table(columns)
///     .row(User { name: "Alice".into(), email: "alice@example.com".into() })
///     .row(User { name: "Bob".into(), email: "bob@example.com".into() });
/// ```
pub fn table<T: Clone, M>(columns: Vec<Column<T>>) -> Table<T, M> {
    Table::new(columns)
}
