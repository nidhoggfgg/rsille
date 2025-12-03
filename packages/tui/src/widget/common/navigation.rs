//! Navigation utilities for selectable widgets
//!
//! This module provides common navigation logic for widgets that contain
//! selectable items (List, Select, RadioGroup, etc.), reducing code duplication.

/// Navigation helper for widgets with selectable items
///
/// This structure encapsulates common navigation patterns like moving to the
/// next/previous item, skipping disabled items, ensuring visibility, etc.
///
/// # Type Parameters
/// * `T` - The type of items being navigated (must implement `Disableable`)
///
/// # Examples
/// ```
/// use tui::widget::common::SelectableNavigation;
///
/// struct MyItem {
///     label: String,
///     disabled: bool,
/// }
///
/// impl Disableable for MyItem {
///     fn is_disabled(&self) -> bool { self.disabled }
/// }
///
/// let items = vec![
///     MyItem { label: "A".into(), disabled: false },
///     MyItem { label: "B".into(), disabled: true },
///     MyItem { label: "C".into(), disabled: false },
/// ];
///
/// let mut nav = SelectableNavigation::new(&items);
/// nav.focus_next(); // Skips disabled item B, goes to C
/// ```
#[derive(Debug, Clone)]
pub struct SelectableNavigation {
    focused_index: Option<usize>,
    scroll_offset: usize,
    viewport_size: usize,
}

impl SelectableNavigation {
    /// Create a new navigation helper
    ///
    /// # Arguments
    /// * `item_count` - Total number of items
    /// * `viewport_size` - Number of items visible at once
    pub fn new(_item_count: usize, viewport_size: usize) -> Self {
        Self {
            focused_index: None,
            scroll_offset: 0,
            viewport_size,
        }
    }

    /// Create with initial focused index
    ///
    /// # Arguments
    /// * `item_count` - Total number of items
    /// * `viewport_size` - Number of items visible at once
    /// * `initial_index` - Initial focused index
    pub fn with_initial_focus(
        item_count: usize,
        viewport_size: usize,
        initial_index: Option<usize>,
    ) -> Self {
        let mut nav = Self::new(item_count, viewport_size);
        nav.focused_index = initial_index;
        nav
    }

    /// Get the currently focused index
    pub fn focused_index(&self) -> Option<usize> {
        self.focused_index
    }

    /// Get the current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set the focused index
    pub fn set_focused_index(&mut self, index: Option<usize>) {
        self.focused_index = index;
    }

    /// Set the scroll offset
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    /// Set the viewport size
    pub fn set_viewport_size(&mut self, size: usize) {
        self.viewport_size = size;
    }

    /// Get the viewport size
    pub fn viewport_size(&self) -> usize {
        self.viewport_size
    }

    /// Move focus to the next non-disabled item
    ///
    /// # Arguments
    /// * `is_disabled` - Closure that returns true if an item at the given index is disabled
    /// * `item_count` - Total number of items
    pub fn focus_next<F>(&mut self, is_disabled: F, item_count: usize)
    where
        F: Fn(usize) -> bool,
    {
        if item_count == 0 {
            return;
        }

        let start_idx = self.focused_index.map(|i| i + 1).unwrap_or(0);

        // Search forward for next non-disabled item
        for offset in 0..item_count {
            let idx = (start_idx + offset) % item_count;
            if !is_disabled(idx) {
                self.focused_index = Some(idx);
                self.ensure_visible(item_count);
                return;
            }
        }
    }

    /// Move focus to the previous non-disabled item
    ///
    /// # Arguments
    /// * `is_disabled` - Closure that returns true if an item at the given index is disabled
    /// * `item_count` - Total number of items
    pub fn focus_previous<F>(&mut self, is_disabled: F, item_count: usize)
    where
        F: Fn(usize) -> bool,
    {
        if item_count == 0 {
            return;
        }

        let start_idx = self
            .focused_index
            .unwrap_or(0)
            .checked_sub(1)
            .unwrap_or(item_count - 1);

        // Search backward for previous non-disabled item
        for offset in 0..item_count {
            let idx = (start_idx + item_count - offset) % item_count;
            if !is_disabled(idx) {
                self.focused_index = Some(idx);
                self.ensure_visible(item_count);
                return;
            }
        }
    }

    /// Move focus to the first non-disabled item
    ///
    /// # Arguments
    /// * `is_disabled` - Closure that returns true if an item at the given index is disabled
    /// * `item_count` - Total number of items
    pub fn focus_first<F>(&mut self, is_disabled: F, item_count: usize)
    where
        F: Fn(usize) -> bool,
    {
        for idx in 0..item_count {
            if !is_disabled(idx) {
                self.focused_index = Some(idx);
                self.ensure_visible(item_count);
                return;
            }
        }
    }

    /// Move focus to the last non-disabled item
    ///
    /// # Arguments
    /// * `is_disabled` - Closure that returns true if an item at the given index is disabled
    /// * `item_count` - Total number of items
    pub fn focus_last<F>(&mut self, is_disabled: F, item_count: usize)
    where
        F: Fn(usize) -> bool,
    {
        for idx in (0..item_count).rev() {
            if !is_disabled(idx) {
                self.focused_index = Some(idx);
                self.ensure_visible(item_count);
                return;
            }
        }
    }

    /// Jump focus forward by a page
    ///
    /// # Arguments
    /// * `is_disabled` - Closure that returns true if an item at the given index is disabled
    /// * `item_count` - Total number of items
    pub fn page_down<F>(&mut self, is_disabled: F, item_count: usize)
    where
        F: Fn(usize) -> bool,
    {
        if item_count == 0 {
            return;
        }

        let page_size = self.viewport_size.saturating_sub(1).max(1);
        let current = self.focused_index.unwrap_or(0);
        let target = (current + page_size).min(item_count - 1);

        // Find nearest non-disabled item forward from target
        for idx in target..item_count {
            if !is_disabled(idx) {
                self.focused_index = Some(idx);
                self.ensure_visible(item_count);
                return;
            }
        }

        // If no non-disabled item found forward, search backward from target
        for idx in (0..=target).rev() {
            if !is_disabled(idx) {
                self.focused_index = Some(idx);
                self.ensure_visible(item_count);
                return;
            }
        }
    }

    /// Jump focus backward by a page
    ///
    /// # Arguments
    /// * `is_disabled` - Closure that returns true if an item at the given index is disabled
    /// * `item_count` - Total number of items
    pub fn page_up<F>(&mut self, is_disabled: F, item_count: usize)
    where
        F: Fn(usize) -> bool,
    {
        if item_count == 0 {
            return;
        }

        let page_size = self.viewport_size.saturating_sub(1).max(1);
        let current = self.focused_index.unwrap_or(0);
        let target = current.saturating_sub(page_size);

        // Find nearest non-disabled item backward from target
        for idx in (0..=target).rev() {
            if !is_disabled(idx) {
                self.focused_index = Some(idx);
                self.ensure_visible(item_count);
                return;
            }
        }

        // If no non-disabled item found backward, search forward from target
        for idx in target..item_count {
            if !is_disabled(idx) {
                self.focused_index = Some(idx);
                self.ensure_visible(item_count);
                return;
            }
        }
    }

    /// Ensure the focused item is visible in the viewport
    ///
    /// Adjusts scroll_offset to ensure focused_index is visible.
    ///
    /// # Arguments
    /// * `item_count` - Total number of items
    pub fn ensure_visible(&mut self, _item_count: usize) {
        if let Some(focused_idx) = self.focused_index {
            // If focused item is above viewport, scroll up
            if focused_idx < self.scroll_offset {
                self.scroll_offset = focused_idx;
            }
            // If focused item is below viewport, scroll down
            else if focused_idx >= self.scroll_offset + self.viewport_size {
                self.scroll_offset = focused_idx.saturating_sub(self.viewport_size - 1);
            }
        }
    }

    /// Check if an index is currently visible in the viewport
    ///
    /// # Arguments
    /// * `index` - The index to check
    pub fn is_visible(&self, index: usize) -> bool {
        index >= self.scroll_offset && index < self.scroll_offset + self.viewport_size
    }

    /// Get the visible range of indices
    ///
    /// # Arguments
    /// * `item_count` - Total number of items
    ///
    /// # Returns
    /// A tuple of (start_index, end_index) representing the visible range
    pub fn visible_range(&self, item_count: usize) -> (usize, usize) {
        let start = self.scroll_offset;
        let end = (start + self.viewport_size).min(item_count);
        (start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_next() {
        let mut nav = SelectableNavigation::new(5, 3);
        nav.set_focused_index(Some(0));

        nav.focus_next(|_| false, 5);
        assert_eq!(nav.focused_index(), Some(1));

        nav.focus_next(|_| false, 5);
        assert_eq!(nav.focused_index(), Some(2));
    }

    #[test]
    fn test_navigation_skip_disabled() {
        let mut nav = SelectableNavigation::new(5, 3);
        nav.set_focused_index(Some(0));

        // Item 1 is disabled
        nav.focus_next(|idx| idx == 1, 5);
        assert_eq!(nav.focused_index(), Some(2)); // Should skip index 1
    }

    #[test]
    fn test_navigation_previous() {
        let mut nav = SelectableNavigation::new(5, 3);
        nav.set_focused_index(Some(2));

        nav.focus_previous(|_| false, 5);
        assert_eq!(nav.focused_index(), Some(1));

        nav.focus_previous(|_| false, 5);
        assert_eq!(nav.focused_index(), Some(0));
    }

    #[test]
    fn test_navigation_first_last() {
        let mut nav = SelectableNavigation::new(5, 3);

        nav.focus_first(|_| false, 5);
        assert_eq!(nav.focused_index(), Some(0));

        nav.focus_last(|_| false, 5);
        assert_eq!(nav.focused_index(), Some(4));
    }

    #[test]
    fn test_ensure_visible() {
        let mut nav = SelectableNavigation::new(10, 3);

        // Focus item beyond viewport
        nav.set_focused_index(Some(5));
        nav.ensure_visible(10);

        // Scroll offset should adjust to make item 5 visible
        assert!(nav.is_visible(5));
    }

    #[test]
    fn test_visible_range() {
        let nav = SelectableNavigation::new(10, 3);
        let (start, end) = nav.visible_range(10);

        assert_eq!(start, 0);
        assert_eq!(end, 3);
    }
}
