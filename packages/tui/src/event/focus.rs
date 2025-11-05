//! Focus management for keyboard navigation

use crate::widget::AnyWidget;

/// Unique identifier for widgets in the focus order
pub type WidgetId = usize;

/// Manages focus state and navigation order for interactive widgets
#[derive(Debug)]
pub struct FocusManager {
    /// IDs of focusable widgets in order
    focus_order: Vec<WidgetId>,
    /// Current focus index (into focus_order)
    current_index: Option<usize>,
}

impl FocusManager {
    /// Create a new FocusManager by scanning the widget tree
    ///
    /// # Examples
    /// ```
    /// use tui::event::focus::FocusManager;
    /// use tui::widget::AnyWidget;
    ///
    /// let widgets: Vec<AnyWidget<()>> = vec![];
    /// let manager = FocusManager::new(&widgets);
    /// ```
    pub fn new<M: Clone>(widgets: &[AnyWidget<M>]) -> Self {
        let focus_order = Self::build_focus_order(widgets);
        let current_index = if focus_order.is_empty() {
            None
        } else {
            Some(0)
        };

        Self {
            focus_order,
            current_index,
        }
    }

    /// Build focus order by traversing widget tree
    fn build_focus_order<M: Clone>(widgets: &[AnyWidget<M>]) -> Vec<WidgetId> {
        let mut order = Vec::new();
        for (id, widget) in widgets.iter().enumerate() {
            if widget.focusable() {
                order.push(id);
            }
        }
        order
    }

    /// Move focus to the next focusable widget (Tab)
    pub fn next(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        self.current_index = Some(match self.current_index {
            Some(idx) => (idx + 1) % self.focus_order.len(),
            None => 0,
        });
    }

    /// Move focus to the previous focusable widget (Shift+Tab)
    pub fn prev(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        self.current_index = Some(match self.current_index {
            Some(0) => self.focus_order.len() - 1,
            Some(idx) => idx - 1,
            None => 0,
        });
    }

    /// Get the currently focused widget ID
    pub fn current(&self) -> Option<WidgetId> {
        self.current_index
            .and_then(|idx| self.focus_order.get(idx).copied())
    }

    /// Get the focused widget index (into the widget array)
    pub fn focused_index(&self) -> Option<usize> {
        self.current()
    }

    /// Check if a widget ID is currently focused
    pub fn is_focused(&self, widget_id: WidgetId) -> bool {
        self.current() == Some(widget_id)
    }

    /// Get the number of focusable widgets
    pub fn len(&self) -> usize {
        self.focus_order.len()
    }

    /// Check if there are any focusable widgets
    pub fn is_empty(&self) -> bool {
        self.focus_order.is_empty()
    }

    /// Rebuild focus order (call after widget tree changes)
    pub fn rebuild<M: Clone>(&mut self, widgets: &[AnyWidget<M>]) {
        let old_current = self.current();
        self.focus_order = Self::build_focus_order(widgets);

        // Try to maintain focus on the same widget
        self.current_index = if self.focus_order.is_empty() {
            None
        } else if let Some(id) = old_current {
            self.focus_order.iter().position(|&x| x == id).or(Some(0))
        } else {
            Some(0)
        };
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self {
            focus_order: Vec::new(),
            current_index: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Button, Label};

    #[test]
    fn test_focus_manager_creation() {
        let widgets: Vec<AnyWidget> = vec![
            Label::new("Not focusable").into(),
            Button::new("Focusable 1").into(),
            Button::new("Focusable 2").into(),
        ];

        let manager = FocusManager::new(&widgets);
        assert_eq!(manager.len(), 2);
        assert_eq!(manager.current(), Some(1)); // First button at index 1
    }

    #[test]
    fn test_focus_navigation() {
        let widgets: Vec<AnyWidget> = vec![
            Button::new("Button 1").into(),
            Button::new("Button 2").into(),
            Button::new("Button 3").into(),
        ];

        let mut manager = FocusManager::new(&widgets);
        assert_eq!(manager.current(), Some(0));

        manager.next();
        assert_eq!(manager.current(), Some(1));

        manager.next();
        assert_eq!(manager.current(), Some(2));

        // Wrap around
        manager.next();
        assert_eq!(manager.current(), Some(0));
    }

    #[test]
    fn test_focus_prev() {
        let widgets: Vec<AnyWidget> = vec![
            Button::new("Button 1").into(),
            Button::new("Button 2").into(),
        ];

        let mut manager = FocusManager::new(&widgets);
        assert_eq!(manager.current(), Some(0));

        // Wrap to end
        manager.prev();
        assert_eq!(manager.current(), Some(1));

        manager.prev();
        assert_eq!(manager.current(), Some(0));
    }

    #[test]
    fn test_empty_focus_manager() {
        let widgets: Vec<AnyWidget> = vec![Label::new("No focusable").into()];

        let mut manager = FocusManager::new(&widgets);
        assert!(manager.is_empty());
        assert_eq!(manager.current(), None);

        manager.next();
        assert_eq!(manager.current(), None);
    }

    #[test]
    fn test_is_focused() {
        let widgets: Vec<AnyWidget> = vec![
            Button::new("Button 1").into(),
            Button::new("Button 2").into(),
        ];

        let manager = FocusManager::new(&widgets);
        assert!(manager.is_focused(0));
        assert!(!manager.is_focused(1));
    }
}
