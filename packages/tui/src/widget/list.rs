//! List widget - interactive scrollable list component
//!
//! A modern list component inspired by contemporary UI frameworks like Shadcn/UI,
//! Chakra UI, and Material UI, adapted for terminal interfaces.

use super::*;
use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::style::{Style, ThemeManager};
use crate::widget::common::SelectableNavigation;
use std::sync::Arc;

/// Selection event information
///
/// Contains both the selected values and the current list state,
/// allowing applications to preserve scroll position when recreating lists.
#[derive(Debug, Clone)]
pub struct SelectionEvent<T: Clone> {
    /// The selected item values
    pub selected_values: Vec<T>,
    /// Current focused index
    pub focused_index: Option<usize>,
    /// Current scroll offset
    pub scroll_offset: usize,
}

/// Selection mode for the list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// No selection allowed - display only
    None,
    /// Single item selection
    #[default]
    Single,
    /// Multiple item selection
    Multiple,
}

/// A single item in the list
#[derive(Debug, Clone)]
pub struct ListItem<T: Clone> {
    /// The actual value/data of this item
    pub value: T,
    /// Display label for this item
    pub label: String,
    /// Whether this item is disabled (cannot be selected)
    pub disabled: bool,
    /// Whether to show a divider line below this item
    pub divider_below: bool,
}

impl<T: Clone> ListItem<T> {
    /// Create a new list item with the given value and label
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            disabled: false,
            divider_below: false,
        }
    }

    /// Mark this item as disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add a divider line below this item
    pub fn with_divider(mut self) -> Self {
        self.divider_below = true;
        self
    }
}

/// Interactive scrollable list widget
///
/// A modern list component with support for:
/// - Single and multiple selection modes
/// - Keyboard navigation (arrows, PageUp/Down, Home/End)
/// - Mouse click selection
/// - Scrollable viewport for large lists
/// - Disabled items
/// - Dividers between items
/// - Empty state message
/// - Custom styling
///
/// # Examples
/// ```
/// use tui::widget::{List, ListItem, SelectionMode};
///
/// #[derive(Clone, Debug, PartialEq)]
/// enum Fruit {
///     Apple,
///     Banana,
///     Orange,
/// }
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     FruitSelected(Vec<Fruit>),
/// }
///
/// let items = vec![
///     ListItem::new(Fruit::Apple, "Apple"),
///     ListItem::new(Fruit::Banana, "Banana").with_divider(),
///     ListItem::new(Fruit::Orange, "Orange"),
/// ];
///
/// let list = List::new(items)
///     .selection_mode(SelectionMode::Multiple)
///     .on_select(|selected| Message::FruitSelected(selected));
/// ```
#[derive(Clone)]
pub struct List<T: Clone, M = ()> {
    items: Vec<ListItem<T>>,
    selected_indices: Vec<usize>,
    selection_mode: SelectionMode,
    navigation: SelectableNavigation,
    empty_message: String,
    show_scrollbar: bool,
    focused: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    custom_selected_style: Option<Style>,
    on_select: Option<Arc<dyn Fn(SelectionEvent<T>) -> M + Send + Sync>>,
}

impl<T: Clone, M> std::fmt::Debug for List<T, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("List")
            .field("items", &self.items.len())
            .field("selected_indices", &self.selected_indices)
            .field("focused_index", &self.navigation.focused_index())
            .field("selection_mode", &self.selection_mode)
            .field("scroll_offset", &self.navigation.scroll_offset())
            .field("viewport_size", &self.navigation.scroll_offset())
            .field("empty_message", &self.empty_message)
            .field("show_scrollbar", &self.show_scrollbar)
            .field("focused", &self.focused)
            .field("on_select", &self.on_select.is_some())
            .finish()
    }
}

impl<T: Clone, M> List<T, M> {
    /// Create a new list with the given items
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem};
    ///
    /// let items = vec![
    ///     ListItem::new("item1", "First Item"),
    ///     ListItem::new("item2", "Second Item"),
    /// ];
    ///
    /// let list = List::<&str, ()>::new(items);
    /// ```
    pub fn new(items: Vec<ListItem<T>>) -> Self {
        // Auto-focus first non-disabled item if any items exist
        let focused_index = if !items.is_empty() {
            items.iter().position(|item| !item.disabled)
        } else {
            None
        };

        let navigation = SelectableNavigation::with_initial_focus(
            items.len(),
            10, // Default viewport height
            focused_index,
        );

        Self {
            items,
            selected_indices: Vec::new(),
            selection_mode: SelectionMode::default(),
            navigation,
            empty_message: "No items".to_string(),
            show_scrollbar: true,
            focused: false,
            custom_style: None,
            custom_focus_style: None,
            custom_selected_style: None,
            on_select: None,
        }
    }

    /// Create an empty list
    ///
    /// Use this with `.item()` for fluent building.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::List;
    ///
    /// let list = List::<&str, ()>::empty()
    ///     .item("a", "Option A")
    ///     .item("b", "Option B")
    ///     .item_disabled("c", "Option C (disabled)");
    /// ```
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Add an item to the list (fluent builder pattern)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::List;
    ///
    /// let list = List::<&str, ()>::empty()
    ///     .item("apple", "🍎 Apple")
    ///     .item("banana", "🍌 Banana");
    /// ```
    pub fn item(mut self, value: T, label: impl Into<String>) -> Self {
        let item = ListItem::new(value, label);
        self.items.push(item);

        // Update focused_index if this is the first non-disabled item
        if self.navigation.focused_index().is_none() {
            self.navigation.set_focused_index(Some(self.items.len() - 1));
        }

        self
    }

    /// Add a disabled item to the list
    ///
    /// # Examples
    /// ```
    /// use tui::widget::List;
    ///
    /// let list = List::<&str, ()>::empty()
    ///     .item("available", "Available")
    ///     .item_disabled("unavailable", "Unavailable");
    /// ```
    pub fn item_disabled(mut self, value: T, label: impl Into<String>) -> Self {
        let item = ListItem::new(value, label).disabled(true);
        self.items.push(item);
        self
    }

    /// Add an item with a divider below it
    ///
    /// # Examples
    /// ```
    /// use tui::widget::List;
    ///
    /// let list = List::<&str, ()>::empty()
    ///     .item("fruits", "Fruits")
    ///     .item_with_divider("apple", "🍎 Apple")
    ///     .item("vegetables", "Vegetables");
    /// ```
    pub fn item_with_divider(mut self, value: T, label: impl Into<String>) -> Self {
        let item = ListItem::new(value, label).with_divider();
        self.items.push(item);

        // Update focused_index if this is the first non-disabled item
        if self.navigation.focused_index().is_none() {
            self.navigation.set_focused_index(Some(self.items.len() - 1));
        }

        self
    }

    /// Add multiple items from an iterator
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem};
    ///
    /// let items = vec![
    ///     ListItem::new("a", "Option A"),
    ///     ListItem::new("b", "Option B"),
    /// ];
    ///
    /// let list = List::<&str, ()>::empty()
    ///     .items(items);
    /// ```
    pub fn items(mut self, items: impl IntoIterator<Item = ListItem<T>>) -> Self {
        for item in items {
            let is_disabled = item.disabled;
            self.items.push(item);

            // Update focused_index if this is the first non-disabled item
            if self.navigation.focused_index().is_none() && !is_disabled {
                self.navigation.set_focused_index(Some(self.items.len() - 1));
            }
        }
        self
    }

    /// Set the selection mode
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem, SelectionMode};
    ///
    /// let list = List::<&str, ()>::new(vec![])
    ///     .selection_mode(SelectionMode::Multiple);
    /// ```
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Set the focused index
    ///
    /// This allows you to restore the focused position when recreating the list.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem};
    ///
    /// let list = List::<&str, ()>::new(vec![])
    ///     .focused_index(Some(5));
    /// ```
    pub fn focused_index(mut self, index: Option<usize>) -> Self {
        // Validate the index
        if let Some(idx) = index {
            if idx < self.items.len() && !self.items[idx].disabled {
                self.navigation.set_focused_index(Some(idx));
                // Ensure the focused item is visible
                self.navigation.ensure_visible(self.items.len());
            }
        }
        self
    }

    /// Set the scroll offset
    ///
    /// This allows you to restore the scroll position when recreating the list.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem};
    ///
    /// let list = List::<&str, ()>::new(vec![])
    ///     .scroll_offset(3);
    /// ```
    pub fn scroll_offset(mut self, offset: usize) -> Self {
        let max_offset = self.items.len().saturating_sub(1);
        self.navigation.set_scroll_offset(offset.min(max_offset));
        self
    }

    /// Get the current focused index
    ///
    /// This allows you to save the focused position before recreating the list.
    pub fn get_focused_index(&self) -> Option<usize> {
        self.navigation.focused_index()
    }

    /// Get the current scroll offset
    ///
    /// This allows you to save the scroll position before recreating the list.
    pub fn get_scroll_offset(&self) -> usize {
        self.navigation.scroll_offset()
    }

    /// Set the viewport height (number of visible items)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem};
    ///
    /// let list = List::<&str, ()>::new(vec![])
    ///     .viewport_height(5);
    /// ```
    pub fn viewport_height(mut self, height: u16) -> Self {
        self.navigation.set_viewport_size(height as usize);
        self
    }

    /// Set the empty state message
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem};
    ///
    /// let list = List::<&str, ()>::new(vec![])
    ///     .empty_message("No items available");
    /// ```
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = message.into();
        self
    }

    /// Set whether to show the scrollbar
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem};
    ///
    /// let list = List::<&str, ()>::new(vec![])
    ///     .show_scrollbar(false);
    /// ```
    pub fn show_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }

    /// Set initially selected indices
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem};
    ///
    /// let list = List::<&str, ()>::new(vec![])
    ///     .selected(vec![0, 2]);
    /// ```
    pub fn selected(mut self, indices: Vec<usize>) -> Self {
        // Filter out invalid indices and disabled items
        self.selected_indices = indices
            .into_iter()
            .filter(|&idx| idx < self.items.len() && !self.items[idx].disabled)
            .collect();
        self
    }

    /// Set the selection change handler
    ///
    /// The handler receives a `SelectionEvent` containing the selected values
    /// and the current list state (focused_index and scroll_offset).
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{List, ListItem};
    ///
    /// #[derive(Clone)]
    /// enum Message { ItemsSelected(Vec<String>, Option<usize>, usize) }
    ///
    /// let list = List::new(vec![])
    ///     .on_select(|event| Message::ItemsSelected(
    ///         event.selected_values,
    ///         event.focused_index,
    ///         event.scroll_offset
    ///     ));
    /// ```
    pub fn on_select<F>(mut self, handler: F) -> Self
    where
        F: Fn(SelectionEvent<T>) -> M + Send + Sync + 'static,
    {
        self.on_select = Some(Arc::new(handler));
        self
    }

    /// Set a custom style (overrides theme styling)
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Set a custom focus style (overrides theme focus styling)
    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Set a custom selected item style (overrides theme selected styling)
    pub fn selected_style(mut self, style: Style) -> Self {
        self.custom_selected_style = Some(style);
        self
    }
}

/// Pre-computed styles for efficient rendering
struct ItemStyles {
    normal: render::style::Style,
    focused: render::style::Style,
    selected: render::style::Style,
    disabled: render::style::Style,
    divider: render::style::Style,
    scrollbar_track: render::style::Style,
    scrollbar_thumb: render::style::Style,
}

impl<T: Clone, M> List<T, M> {
    /// Get pre-computed styles for all item states (called once per render)
    fn get_item_styles(&self) -> ItemStyles {
        ThemeManager::global().with_theme(|theme| {
            // Normal item style - use text style without background for clean look
            let normal = self
                .custom_style
                .unwrap_or(Style::default().fg(theme.colors.text))
                .to_render_style();

            // Focused item style - highlight with focus ring color and subtle background
            let base_focused = Style::default()
                .fg(theme.colors.text)
                .bg(theme.colors.focus_background);
            let focused = self
                .custom_focus_style
                .as_ref()
                .map(|s| s.merge(base_focused))
                .unwrap_or(base_focused)
                .to_render_style();

            // Selected item style - use primary color background
            let base_selected = Style::default()
                .fg(theme.colors.text)
                .bg(theme.colors.primary);
            let selected = self
                .custom_selected_style
                .as_ref()
                .map(|s| s.merge(base_selected))
                .unwrap_or(base_selected)
                .to_render_style();

            // Disabled item style - muted appearance
            let disabled = Style::default()
                .fg(theme.colors.text_muted)
                .to_render_style();

            // Divider style
            let divider = Style::default().fg(theme.colors.border).to_render_style();

            // Scrollbar styles
            let scrollbar_track = Style::default().fg(theme.colors.border).to_render_style();
            let scrollbar_thumb = Style::default().fg(theme.colors.info).to_render_style();

            ItemStyles {
                normal,
                focused,
                selected,
                disabled,
                divider,
                scrollbar_track,
                scrollbar_thumb,
            }
        })
    }

    /// Get the effective style for a specific item
    #[inline]
    fn select_item_style(
        &self,
        styles: &ItemStyles,
        is_disabled: bool,
        is_focused: bool,
        is_selected: bool,
    ) -> render::style::Style {
        if is_disabled {
            styles.disabled
        } else if is_selected {
            styles.selected
        } else if is_focused && self.focused {
            styles.focused
        } else {
            styles.normal
        }
    }

    /// Calculate total rows needed including dividers
    fn calculate_total_rows(&self) -> usize {
        let divider_count = self.items.iter().filter(|item| item.divider_below).count();
        self.items.len() + divider_count
    }

    /// Toggle selection of the focused item
    fn toggle_selection(&mut self) -> Vec<M> {
        if self.selection_mode == SelectionMode::None {
            return vec![];
        }

        let Some(focused_idx) = self.navigation.focused_index() else {
            return vec![];
        };

        // Don't allow selecting disabled items
        if self.items[focused_idx].disabled {
            return vec![];
        }

        match self.selection_mode {
            SelectionMode::Single => {
                // Toggle: if already selected, deselect; otherwise select only this item
                if self.selected_indices.contains(&focused_idx) {
                    self.selected_indices.clear();
                } else {
                    self.selected_indices = vec![focused_idx];
                }
            }
            SelectionMode::Multiple => {
                // Toggle: add or remove from selection
                if let Some(pos) = self
                    .selected_indices
                    .iter()
                    .position(|&idx| idx == focused_idx)
                {
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
            let selected_values: Vec<T> = self
                .selected_indices
                .iter()
                .filter_map(|&idx| self.items.get(idx).map(|item| item.value.clone()))
                .collect();

            let event = SelectionEvent {
                selected_values,
                focused_index: self.navigation.focused_index(),
                scroll_offset: self.navigation.scroll_offset(),
            };

            let message = handler(event);
            vec![message]
        } else {
            vec![]
        }
    }
}

impl<T: Clone + Send + Sync, M: Send + Sync> Widget<M> for List<T, M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let width = area.width();
        let height = area.height();

        // Handle empty state
        if self.items.is_empty() {
            let style = ThemeManager::global()
                .with_theme(|theme| theme.styles.text_muted.to_render_style());

            use unicode_width::UnicodeWidthStr;
            let msg_width = self.empty_message.width() as u16;
            let x = if width > msg_width {
                (width - msg_width) / 2
            } else {
                0
            };
            let y = height / 2;

            let _ = chunk.set_string(x, y, &self.empty_message, style);
            return;
        }

        // Pre-compute all styles once (performance optimization)
        let item_styles = self.get_item_styles();

        // Calculate visible range
        let viewport_size = height.min(self.navigation.viewport_size() as u16) as usize;
        let visible_start = self.navigation.scroll_offset();
        let visible_end = (visible_start + viewport_size).min(self.items.len());

        // Determine scrollbar width - consider dividers when calculating total rows
        let total_rows = self.calculate_total_rows();
        let scrollbar_width = if self.show_scrollbar && total_rows > viewport_size {
            2 // "│ " for scrollbar
        } else {
            0
        };
        let content_width = width.saturating_sub(scrollbar_width);

        // Render visible items
        let mut y = 0u16;
        for (list_idx, item) in self
            .items
            .iter()
            .enumerate()
            .skip(visible_start)
            .take(visible_end - visible_start)
        {
            if y >= height {
                break;
            }

            let is_item_focused = Some(list_idx) == self.navigation.focused_index();
            let is_selected = self.selected_indices.contains(&list_idx);

            // Get the appropriate style for this item
            let render_style =
                self.select_item_style(&item_styles, item.disabled, is_item_focused, is_selected);

            // Render selection/focus indicator
            let indicator = match self.selection_mode {
                SelectionMode::Multiple => {
                    if is_selected {
                        "[✓] "
                    } else {
                        "[ ] "
                    }
                }
                SelectionMode::Single => {
                    if is_selected {
                        "(•) "
                    } else {
                        "( ) "
                    }
                }
                SelectionMode::None => {
                    if is_item_focused && self.focused {
                        "> "
                    } else {
                        "  "
                    }
                }
            };

            // Always render background for consistent appearance
            // For focused/selected items, fill with their style
            // For normal items, fill with normal style (which has no bg, keeping terminal default)
            if is_item_focused || is_selected {
                let _ = chunk.fill(0, y, content_width, 1, ' ', render_style);
            }

            // Render indicator
            let _ = chunk.set_string(0, y, indicator, render_style);

            // Render item label (use string slice to avoid cloning)
            use unicode_width::UnicodeWidthStr;
            let indicator_width = indicator.width() as u16;
            let available_width = content_width.saturating_sub(indicator_width);

            // Truncate label if needed (without cloning unless necessary)
            if item.label.width() as u16 > available_width {
                // Only allocate new string if truncation is needed
                let mut truncated = String::with_capacity(available_width as usize);
                let mut current_width = 0;
                for ch in item.label.chars() {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                    if current_width + ch_width + 1 > available_width {
                        // +1 for ellipsis
                        truncated.push('…');
                        break;
                    }
                    truncated.push(ch);
                    current_width += ch_width;
                }
                let _ = chunk.set_string(indicator_width, y, &truncated, render_style);
            } else {
                // Use string directly without cloning
                let _ = chunk.set_string(indicator_width, y, &item.label, render_style);
            }

            y += 1;

            // Render divider if needed
            if item.divider_below && y < height {
                // Draw horizontal line with pre-computed divider style
                for x in 0..content_width {
                    let _ = chunk.set_char(x, y, '─', item_styles.divider);
                }
                y += 1;
            }
        }

        // Render scrollbar if needed
        if self.show_scrollbar && total_rows > viewport_size {
            self.render_scrollbar(chunk, content_width, height, viewport_size, &item_styles);
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Down => {
                    self.navigation.focus_next(|idx| self.items[idx].disabled, self.items.len());
                    EventResult::Consumed(vec![])
                }
                KeyCode::Up => {
                    self.navigation.focus_previous(|idx| self.items[idx].disabled, self.items.len());
                    EventResult::Consumed(vec![])
                }
                KeyCode::Home => {
                    self.navigation.focus_first(|idx| self.items[idx].disabled, self.items.len());
                    EventResult::Consumed(vec![])
                }
                KeyCode::End => {
                    self.navigation.focus_last(|idx| self.items[idx].disabled, self.items.len());
                    EventResult::Consumed(vec![])
                }
                KeyCode::PageDown => {
                    self.navigation.page_down(|idx| self.items[idx].disabled, self.items.len());
                    EventResult::Consumed(vec![])
                }
                KeyCode::PageUp => {
                    self.navigation.page_up(|idx| self.items[idx].disabled, self.items.len());
                    EventResult::Consumed(vec![])
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let messages = self.toggle_selection();
                    EventResult::Consumed(messages)
                }
                _ => EventResult::Ignored,
            },
            Event::Mouse(mouse_event) => {
                // Basic mouse support - would need area context for proper implementation
                match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let messages = self.toggle_selection();
                        EventResult::Consumed(messages)
                    }
                    MouseEventKind::ScrollDown => {
                        self.navigation.focus_next(|idx| self.items[idx].disabled, self.items.len());
                        EventResult::Consumed(vec![])
                    }
                    MouseEventKind::ScrollUp => {
                        self.navigation.focus_previous(|idx| self.items[idx].disabled, self.items.len());
                        EventResult::Consumed(vec![])
                    }
                    _ => EventResult::Ignored,
                }
            }
            _ => EventResult::Ignored,
        }
    }

    fn constraints(&self) -> Constraints {
        use unicode_width::UnicodeWidthStr;

        // Calculate max label width
        let max_label_width = self
            .items
            .iter()
            .map(|item| item.label.width() as u16)
            .max()
            .unwrap_or(self.empty_message.width() as u16);

        // Indicator width (e.g., "[✓] " = 4 chars)
        let indicator_width = 4;

        // Calculate total height (including dividers)
        let content_height = if self.items.is_empty() {
            1 // For empty message
        } else {
            self.calculate_total_rows()
        };

        // Scrollbar width - only reserve space if content exceeds viewport
        let needs_scrollbar = self.show_scrollbar && content_height > self.navigation.viewport_size();
        let scrollbar_width = if needs_scrollbar { 2 } else { 0 };

        let total_width = max_label_width + indicator_width + scrollbar_width;

        let display_height = content_height.min(self.navigation.viewport_size()) as u16;

        Constraints {
            min_width: total_width.max(20), // Minimum reasonable width
            max_width: Some(total_width.max(20)),
            min_height: display_height.max(3),
            max_height: Some(display_height.max(3)),
            flex: None,
        }
    }

    fn focusable(&self) -> bool {
        // List is focusable if it has at least one non-disabled item
        self.items.iter().any(|item| !item.disabled)
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;

        // When gaining focus, ensure we have a focused item
        if focused && self.navigation.focused_index().is_none() && !self.items.is_empty() {
            self.navigation.focus_first(|idx| self.items[idx].disabled, self.items.len());
        }
    }
}

impl<T: Clone, M> List<T, M> {
    /// Render scrollbar in the rightmost column
    fn render_scrollbar(
        &self,
        chunk: &mut render::chunk::Chunk,
        x_offset: u16,
        height: u16,
        viewport_size: usize,
        styles: &ItemStyles,
    ) {
        let total_items = self.items.len();

        // Calculate scrollbar thumb position and size
        let scrollbar_height = height as usize;
        let thumb_size = ((viewport_size as f64 / total_items as f64) * scrollbar_height as f64)
            .max(1.0)
            .round() as usize;

        let scroll_ratio = self.navigation.scroll_offset() as f64 / (total_items - viewport_size).max(1) as f64;
        let thumb_position =
            (scroll_ratio * (scrollbar_height - thumb_size) as f64).round() as usize;

        // Draw scrollbar track
        for y in 0..height {
            let _ = chunk.set_char(x_offset, y, '│', styles.scrollbar_track);
        }

        // Draw scrollbar thumb
        for offset in 0..thumb_size {
            let y = (thumb_position + offset).min(scrollbar_height - 1);
            let _ = chunk.set_char(x_offset, y as u16, '█', styles.scrollbar_thumb);
        }
    }
}

/// Create a new list widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message { Selected(SelectionEvent<String>) }
///
/// // Fluent builder style
/// let list = list()
///     .item("a", "Option A")
///     .item("b", "Option B")
///     .item_with_divider("c", "Option C")
///     .on_select(|event| Message::Selected(event));
///
/// // Or with pre-built items
/// let items = vec![
///     ListItem::new("a", "Option A"),
///     ListItem::new("b", "Option B"),
/// ];
/// let list = list().items(items);
/// ```
pub fn list<T: Clone, M>() -> List<T, M> {
    List::empty()
}
