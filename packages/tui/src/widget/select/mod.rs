//! Select widget - dropdown selection component
//!
//! A modern select/dropdown component inspired by contemporary UI frameworks
//! like Shadcn/UI and Chakra UI, adapted for terminal interfaces.

mod item;
mod render;
mod style;

pub use item::{SelectEvent, SelectItem};

use super::*;
use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::style::Style;
use std::sync::{Arc, RwLock};
use style::StyleConfig;

// Import render types from external crate
use ::render::area::Area;
use ::render::chunk::Chunk;

/// Interactive select/dropdown widget
///
/// A modern select component with support for:
/// - Keyboard navigation (arrows, Enter, Escape)
/// - Mouse click selection
/// - Scrollable dropdown menu for large lists
/// - Disabled items
/// - Placeholder text
/// - Custom styling
pub struct Select<T: Clone, M = ()> {
    items: Vec<SelectItem<T>>,
    selected_index: Option<usize>,
    focused_index: Option<usize>,
    placeholder: String,
    opened: bool,
    scroll_offset: usize,
    dropdown_height: u16,
    trigger_focused: bool,
    dropdown_focused: bool,
    overlay_mode: bool,
    borderless: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_select: Option<Arc<dyn Fn(SelectEvent<T>) -> M + Send + Sync>>,
    /// Cached trigger area for overlay positioning
    trigger_area: RwLock<Option<Area>>,
}

// Manual Clone implementation because RwLock doesn't implement Clone
impl<T: Clone, M> Clone for Select<T, M> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            selected_index: self.selected_index,
            focused_index: self.focused_index,
            placeholder: self.placeholder.clone(),
            opened: self.opened,
            scroll_offset: self.scroll_offset,
            dropdown_height: self.dropdown_height,
            trigger_focused: self.trigger_focused,
            dropdown_focused: self.dropdown_focused,
            overlay_mode: self.overlay_mode,
            borderless: self.borderless,
            custom_style: self.custom_style,
            custom_focus_style: self.custom_focus_style,
            on_select: self.on_select.clone(),
            trigger_area: RwLock::new(self.trigger_area.read().ok().and_then(|a| *a)),
        }
    }
}

impl<T: Clone, M> std::fmt::Debug for Select<T, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("items", &self.items.len())
            .field("selected_index", &self.selected_index)
            .field("focused_index", &self.focused_index)
            .field("placeholder", &self.placeholder)
            .field("opened", &self.opened)
            .field("borderless", &self.borderless)
            .field("on_select", &self.on_select.is_some())
            .finish()
    }
}

impl<T: Clone + 'static, M> Select<T, M> {
    /// Create a new select with the given items
    pub fn new(items: Vec<SelectItem<T>>) -> Self {
        // Auto-focus first non-disabled item when dropdown opens
        let focused_index = if !items.is_empty() {
            items.iter().position(|item| !item.disabled)
        } else {
            None
        };

        Self {
            items,
            selected_index: None,
            focused_index,
            placeholder: "Select...".to_string(),
            opened: false,
            scroll_offset: 0,
            dropdown_height: 8,
            trigger_focused: false,
            dropdown_focused: false,
            overlay_mode: true,
            borderless: false,
            custom_style: None,
            custom_focus_style: None,
            on_select: None,
            trigger_area: RwLock::new(None),
        }
    }

    /// Create an empty select
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Add an item to the select (fluent builder pattern)
    pub fn item(mut self, value: T, label: impl Into<String>) -> Self {
        let item = SelectItem::new(value, label);
        self.items.push(item);

        if self.focused_index.is_none() {
            self.focused_index = Some(self.items.len() - 1);
        }

        self
    }

    /// Add a disabled item to the select
    pub fn item_disabled(mut self, value: T, label: impl Into<String>) -> Self {
        let item = SelectItem::new(value, label).disabled(true);
        self.items.push(item);
        self
    }

    /// Set the placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the initially selected item by index
    pub fn selected(mut self, index: Option<usize>) -> Self {
        if let Some(idx) = index {
            if idx < self.items.len() && !self.items[idx].disabled {
                self.selected_index = Some(idx);
            }
        }
        self
    }

    /// Set the dropdown height (number of visible items)
    pub fn dropdown_height(mut self, height: u16) -> Self {
        self.dropdown_height = height;
        self
    }

    /// Set the selection change handler
    pub fn on_select<F>(mut self, handler: F) -> Self
    where
        F: Fn(SelectEvent<T>) -> M + Send + Sync + 'static,
    {
        self.on_select = Some(Arc::new(handler));
        self
    }

    /// Set a custom style
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Set a custom focus style
    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Set overlay mode
    pub fn overlay_mode(mut self, enabled: bool) -> Self {
        self.overlay_mode = enabled;
        self
    }

    /// Set borderless mode
    pub fn borderless(mut self, enabled: bool) -> Self {
        self.borderless = enabled;
        self
    }

    /// Get the currently selected value, if any
    pub fn get_selected(&self) -> Option<&T> {
        self.selected_index
            .and_then(|idx| self.items.get(idx))
            .map(|item| &item.value)
    }

    /// Get style configuration based on borderless mode
    fn style_config(&self) -> StyleConfig {
        StyleConfig::from_borderless(self.borderless)
    }

    /// Get the display text for the trigger
    fn get_trigger_text(&self) -> String {
        if let Some(idx) = self.selected_index {
            if let Some(item) = self.items.get(idx) {
                return item.label.clone();
            }
        }
        self.placeholder.clone()
    }

    /// Toggle the dropdown open/closed state
    fn toggle_dropdown(&mut self) {
        self.opened = !self.opened;
        if self.opened {
            // When opening, reset focused index to selected or first item
            if let Some(selected) = self.selected_index {
                self.focused_index = Some(selected);
            } else if self.focused_index.is_none() {
                self.focused_index = self.items.iter().position(|item| !item.disabled);
            }
            self.ensure_focused_visible();
        }
    }

    /// Close the dropdown
    fn close_dropdown(&mut self) {
        self.opened = false;
    }

    /// Move focus to the next non-disabled item in dropdown
    fn focus_next(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let start_idx = self.focused_index.map(|i| i + 1).unwrap_or(0);

        // Search forward for next non-disabled item
        for offset in 0..self.items.len() {
            let idx = (start_idx + offset) % self.items.len();
            if !self.items[idx].disabled {
                self.focused_index = Some(idx);
                self.ensure_focused_visible();
                return;
            }
        }
    }

    /// Move focus to the previous non-disabled item in dropdown
    fn focus_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let start_idx = self
            .focused_index
            .unwrap_or(0)
            .checked_sub(1)
            .unwrap_or(self.items.len() - 1);

        // Search backward for previous non-disabled item
        for offset in 0..self.items.len() {
            let idx = (start_idx + self.items.len() - offset) % self.items.len();
            if !self.items[idx].disabled {
                self.focused_index = Some(idx);
                self.ensure_focused_visible();
                return;
            }
        }
    }

    /// Ensure the focused item is visible in the dropdown viewport
    fn ensure_focused_visible(&mut self) {
        if let Some(focused_idx) = self.focused_index {
            let viewport_size = self.dropdown_height as usize;

            // If focused item is above viewport, scroll up
            if focused_idx < self.scroll_offset {
                self.scroll_offset = focused_idx;
            }
            // If focused item is below viewport, scroll down
            else if focused_idx >= self.scroll_offset + viewport_size {
                self.scroll_offset = focused_idx.saturating_sub(viewport_size - 1);
            }
        }
    }

    /// Select the currently focused item
    fn select_focused(&mut self) -> Vec<M> {
        let Some(focused_idx) = self.focused_index else {
            return vec![];
        };

        // Don't allow selecting disabled items
        if self.items[focused_idx].disabled {
            return vec![];
        }

        self.selected_index = Some(focused_idx);
        self.opened = false;

        // Emit selection event
        if let Some(ref handler) = self.on_select {
            if let Some(item) = self.items.get(focused_idx) {
                let event = SelectEvent {
                    value: item.value.clone(),
                    index: focused_idx,
                };
                let message = handler(event);
                return vec![message];
            }
        }

        vec![]
    }

    /// Render the trigger button
    fn render_trigger(&self, chunk: &mut Chunk) {
        let config = self.style_config();
        let style = style::get_trigger_style(
            self.trigger_focused,
            self.custom_style,
            self.custom_focus_style,
        );
        let trigger_text = self.get_trigger_text();

        render::render_trigger(chunk, &trigger_text, self.opened, &config, style);
    }

    /// Render the dropdown menu
    fn render_dropdown(&self, chunk: &mut Chunk) {
        let config = self.style_config();
        let styles = style::DropdownStyles::from_theme();

        let area = chunk.area();
        let visible_height = area
            .height()
            .saturating_sub(config.dropdown_border_offset()) as usize;
        let show_scrollbar = self.items.len() > visible_height;

        let scrollbar_x = if self.borderless {
            area.width() - 1
        } else {
            area.width() - 2
        };

        render::render_dropdown(
            chunk,
            &self.items,
            self.focused_index,
            self.selected_index,
            self.scroll_offset,
            &config,
            &styles,
            show_scrollbar,
            scrollbar_x,
        );
    }

    /// Register dropdown as an overlay for later rendering
    fn register_dropdown_overlay(&self, trigger_area: Area)
    where
        T: Send,
    {
        use crate::layout::{OverlayInfo, OverlayManager};

        let config = self.style_config();
        let dropdown_height =
            (self.items.len() as u16).min(self.dropdown_height) + config.dropdown_border_offset();
        let dropdown_area = Area::new(
            (trigger_area.x(), trigger_area.y() + trigger_area.height()).into(),
            (trigger_area.width(), dropdown_height).into(),
        );

        // Clone necessary data for the closure
        let items = self.items.clone();
        let scroll_offset = self.scroll_offset;
        let focused_index = self.focused_index;
        let selected_index = self.selected_index;
        let borderless = self.borderless;

        let overlay = OverlayInfo::new(dropdown_area, 100, move |chunk| {
            let config = StyleConfig::from_borderless(borderless);
            let styles = style::DropdownStyles::from_theme();

            let area = chunk.area();
            let visible_height =
                area.height()
                    .saturating_sub(config.dropdown_border_offset()) as usize;
            let show_scrollbar = items.len() > visible_height;

            let scrollbar_x = if borderless {
                area.width() - 1
            } else {
                area.width() - 2
            };

            render::render_dropdown(
                chunk,
                &items,
                focused_index,
                selected_index,
                scroll_offset,
                &config,
                &styles,
                show_scrollbar,
                scrollbar_x,
            );
        });

        OverlayManager::global().add_overlay(overlay);
    }
}

impl<T: Clone + Send + Sync + 'static, M: Send + Sync> Widget<M> for Select<T, M> {
    fn render(&self, chunk: &mut Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let config = self.style_config();
        let trigger_height = config.trigger_height();

        // Render the trigger button
        let trigger_area = Area::new(
            (area.x(), area.y()).into(),
            (area.width(), trigger_height).into(),
        );

        if let Ok(mut trigger_chunk) = chunk.from_area(trigger_area) {
            self.render_trigger(&mut trigger_chunk);
        }

        // Cache trigger area for overlay positioning
        if let Ok(mut cached_area) = self.trigger_area.write() {
            *cached_area = Some(trigger_area);
        }

        // Render dropdown
        if self.opened {
            if self.overlay_mode {
                // Overlay mode: register dropdown for later rendering
                self.register_dropdown_overlay(trigger_area);
            } else {
                // Inline mode: render dropdown directly below trigger
                let dropdown_y = area.y() + trigger_height;
                let dropdown_height = (self.items.len() as u16).min(self.dropdown_height)
                    + config.dropdown_border_offset();

                if dropdown_y + dropdown_height <= area.y() + area.height() {
                    if let Ok(mut dropdown_chunk) = chunk.from_area(Area::new(
                        (area.x(), dropdown_y).into(),
                        (area.width(), dropdown_height).into(),
                    )) {
                        self.render_dropdown(&mut dropdown_chunk);
                    }
                }
            }
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        match event {
            Event::Key(key_event) => {
                if self.opened {
                    // Dropdown is open - handle navigation
                    match key_event.code {
                        KeyCode::Down => {
                            self.focus_next();
                            EventResult::Consumed(vec![])
                        }
                        KeyCode::Up => {
                            self.focus_previous();
                            EventResult::Consumed(vec![])
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            let messages = self.select_focused();
                            EventResult::Consumed(messages)
                        }
                        KeyCode::Esc => {
                            self.close_dropdown();
                            EventResult::Consumed(vec![])
                        }
                        _ => EventResult::Ignored,
                    }
                } else {
                    // Dropdown is closed - handle trigger activation
                    match key_event.code {
                        KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Down => {
                            self.toggle_dropdown();
                            EventResult::Consumed(vec![])
                        }
                        _ => EventResult::Ignored,
                    }
                }
            }
            Event::Mouse(mouse_event) => {
                if let MouseEventKind::Down(MouseButton::Left) = mouse_event.kind {
                    self.handle_mouse_click(mouse_event)
                } else if let MouseEventKind::Down(MouseButton::Right) = mouse_event.kind {
                    // Right click closes dropdown
                    if self.opened {
                        self.close_dropdown();
                        EventResult::Consumed(vec![])
                    } else {
                        EventResult::Ignored
                    }
                } else {
                    EventResult::Ignored
                }
            }
            _ => EventResult::Ignored,
        }
    }

    fn constraints(&self) -> Constraints {
        use unicode_width::UnicodeWidthStr;

        let config = self.style_config();

        // Calculate max label width
        let max_label_width = self
            .items
            .iter()
            .map(|item| item.label.width() as u16)
            .max()
            .unwrap_or(self.placeholder.width() as u16);

        // Calculate minimum width
        let min_width = if self.borderless {
            max_label_width + 4
        } else {
            max_label_width + 6
        };

        // Calculate height based on mode and state
        let height = if self.overlay_mode {
            config.trigger_height()
        } else if self.opened {
            let dropdown_height = (self.items.len() as u16).min(self.dropdown_height)
                + config.dropdown_border_offset();
            config.trigger_height() + dropdown_height
        } else {
            config.trigger_height()
        };

        Constraints {
            min_width: min_width.max(20),
            max_width: Some(min_width.max(20)),
            min_height: height,
            max_height: Some(height),
            flex: None,
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn is_focused(&self) -> bool {
        self.trigger_focused || self.dropdown_focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.trigger_focused = focused;
        if !focused {
            self.opened = false;
        }
    }
}

// Mouse event handling (separate impl for clarity)
impl<T: Clone + 'static, M> Select<T, M> {
    fn handle_mouse_click(&mut self, mouse_event: &crate::event::MouseEvent) -> EventResult<M> {
        let trigger_area = self.trigger_area.read().ok().and_then(|a| *a);

        let Some(trigger_area) = trigger_area else {
            return EventResult::Ignored;
        };

        // Check if click is on trigger
        let is_trigger_click = mouse_event.column >= trigger_area.x()
            && mouse_event.column < trigger_area.x() + trigger_area.width()
            && mouse_event.row >= trigger_area.y()
            && mouse_event.row < trigger_area.y() + trigger_area.height();

        if is_trigger_click {
            self.toggle_dropdown();
            return EventResult::Consumed(vec![]);
        }

        // If dropdown is open, check if click is on dropdown
        if self.opened {
            let config = self.style_config();
            let dropdown_y = if self.overlay_mode {
                trigger_area.y() + trigger_area.height()
            } else {
                trigger_area.y() + config.trigger_height()
            };
            let dropdown_height = (self.items.len() as u16).min(self.dropdown_height)
                + config.dropdown_border_offset();

            let is_dropdown_click = mouse_event.column >= trigger_area.x()
                && mouse_event.column < trigger_area.x() + trigger_area.width()
                && mouse_event.row >= dropdown_y
                && mouse_event.row < dropdown_y + dropdown_height;

            if is_dropdown_click {
                // Calculate which item was clicked
                let border_adjust = config.border_offset;
                let relative_y = mouse_event.row.saturating_sub(dropdown_y + border_adjust);
                let clicked_idx = self.scroll_offset + relative_y as usize;

                if clicked_idx < self.items.len() {
                    self.focused_index = Some(clicked_idx);
                    let messages = self.select_focused();
                    return EventResult::Consumed(messages);
                }
            } else {
                // Click outside dropdown closes it
                self.close_dropdown();
                return EventResult::Consumed(vec![]);
            }
        }

        EventResult::Ignored
    }
}

/// Create a new select widget (convenience function)
pub fn select<T: Clone + 'static, M>() -> Select<T, M> {
    Select::empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_creation() {
        let items = vec![
            SelectItem::new("a", "Option A"),
            SelectItem::new("b", "Option B"),
        ];

        let select: Select<&str, ()> = Select::new(items);
        assert_eq!(select.items.len(), 2);
        assert_eq!(select.opened, false);
        assert_eq!(select.selected_index, None);
    }

    #[test]
    fn test_select_fluent_api() {
        let select: Select<&str, ()> = select()
            .item("a", "Option A")
            .item("b", "Option B")
            .item("c", "Option C");

        assert_eq!(select.items.len(), 3);
    }

    #[test]
    fn test_select_toggle_dropdown() {
        let mut select: Select<&str, ()> = select().item("a", "Option A").item("b", "Option B");

        assert_eq!(select.opened, false);
        select.toggle_dropdown();
        assert_eq!(select.opened, true);
        select.toggle_dropdown();
        assert_eq!(select.opened, false);
    }

    #[test]
    fn test_select_keyboard_navigation() {
        let mut select: Select<&str, ()> = select()
            .item("a", "Option A")
            .item("b", "Option B")
            .item("c", "Option C");

        select.toggle_dropdown();
        assert_eq!(select.focused_index, Some(0));

        select.focus_next();
        assert_eq!(select.focused_index, Some(1));

        select.focus_next();
        assert_eq!(select.focused_index, Some(2));

        select.focus_previous();
        assert_eq!(select.focused_index, Some(1));
    }

    #[test]
    fn test_select_disabled_items() {
        let mut select: Select<&str, ()> = select()
            .item("a", "Option A")
            .item_disabled("b", "Option B (disabled)")
            .item("c", "Option C");

        select.toggle_dropdown();
        assert_eq!(select.focused_index, Some(0));

        select.focus_next();
        assert_eq!(select.focused_index, Some(2));
    }
}
