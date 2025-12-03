//! Focus management system
//!
//! Provides keyboard focus navigation with Tab/Shift+Tab support.
//!
//! # Performance Optimizations
//!
//! - **O(1) lookups**: Uses WidgetId with integer-based identity
//! - **O(1) navigation**: HashMap-based index for instant focus switching
//! - **Zero heap allocations**: SmallVec for widget paths (depth < 8)

use crate::widget_id::WidgetId;
use rustc_hash::FxHashMap;

/// Focus manager for handling keyboard focus navigation
///
/// Optimized for fast focus switching and lookups:
/// - `focus_next/prev`: O(1) using index mapping
/// - `is_focused`: O(1) using ID comparison
/// - `is_focus_within`: O(n) where n is path depth (typically < 8)
#[derive(Debug, Clone)]
pub struct FocusManager {
    /// Current focused widget ID
    focus_id: Option<WidgetId>,

    /// Focus chain: ordered list of focusable widgets
    focus_chain: Vec<WidgetId>,

    /// Fast index lookup: widget ID -> index in focus_chain
    /// Enables O(1) focus navigation instead of O(n) linear search
    id_to_index: FxHashMap<WidgetId, usize>,
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            focus_id: None,
            focus_chain: Vec::new(),
            id_to_index: FxHashMap::default(),
        }
    }

    /// Focus the next widget (Tab key)
    ///
    /// O(1) operation using index mapping
    pub fn focus_next(&mut self) {
        if self.focus_chain.is_empty() {
            return;
        }

        let current_idx = self.current_index();
        let next_idx = (current_idx + 1) % self.focus_chain.len();
        self.focus_id = Some(self.focus_chain[next_idx].clone());
    }

    /// Focus the previous widget (Shift+Tab)
    ///
    /// O(1) operation using index mapping
    pub fn focus_prev(&mut self) {
        if self.focus_chain.is_empty() {
            return;
        }

        let current_idx = self.current_index();
        let prev_idx = if current_idx == 0 {
            self.focus_chain.len() - 1
        } else {
            current_idx - 1
        };
        self.focus_id = Some(self.focus_chain[prev_idx].clone());
    }

    /// Check if the given widget ID is focused
    ///
    /// O(1) operation
    #[inline(always)]
    pub fn is_focused(&self, id: &WidgetId) -> bool {
        self.focus_id.as_ref() == Some(id)
    }

    /// Check if the given widget path is focused
    ///
    /// This is for backward compatibility with path-based APIs.
    /// O(n) where n is path depth
    pub fn is_path_focused(&self, path: &[usize]) -> bool {
        self.focus_id
            .as_ref()
            .is_some_and(|id| id.path() == path)
    }

    /// Check if focus is within the given path (for containers)
    ///
    /// Returns true if the focused widget is a descendant of the given path.
    /// O(n) where n is path depth (typically < 8)
    pub fn is_focus_within(&self, path: &[usize]) -> bool {
        self.focus_id
            .as_ref()
            .is_some_and(|id| id.is_descendant_of(path))
    }

    /// Get current focused widget ID
    #[inline(always)]
    pub fn focus_id(&self) -> Option<WidgetId> {
        self.focus_id.clone()
    }

    /// Get current focus path (for backward compatibility)
    pub fn focus_path(&self) -> Option<&[usize]> {
        self.focus_id.as_ref().map(|id| id.path())
    }

    /// Set focus chain (called after rebuilding widget tree)
    ///
    /// Builds index mapping for O(1) focus navigation.
    /// O(n) operation where n is number of focusable widgets.
    pub fn set_focus_chain(&mut self, chain: Vec<WidgetId>) {
        // Build index mapping for O(1) lookups
        self.id_to_index.clear();
        self.id_to_index.reserve(chain.len());
        for (index, id) in chain.iter().enumerate() {
            self.id_to_index.insert(id.clone(), index);
        }

        self.focus_chain = chain;

        // Validate and fix focus if needed
        if let Some(ref focused_id) = self.focus_id {
            // O(1) lookup instead of O(n) linear search
            if !self.id_to_index.contains_key(focused_id) {
                // Current focus is invalid, focus first widget
                self.focus_id = self.focus_chain.first().cloned();
            }
        } else if !self.focus_chain.is_empty() {
            // Auto-focus first widget if nothing is focused
            self.focus_id = self.focus_chain.first().cloned();
        }
    }

    /// Clear focus
    #[inline]
    pub fn clear_focus(&mut self) {
        self.focus_id = None;
    }

    /// Get focus chain
    #[inline]
    pub fn focus_chain(&self) -> &[WidgetId] {
        &self.focus_chain
    }

    /// Get current index in focus chain
    ///
    /// O(1) operation using index mapping (was O(n) before)
    #[inline]
    fn current_index(&self) -> usize {
        self.focus_id
            .as_ref()
            .and_then(|id| self.id_to_index.get(id).copied())
            .unwrap_or(0)
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::SmallVec;

    fn make_id(path: &[usize]) -> WidgetId {
        WidgetId::from_path(SmallVec::from_slice(path))
    }

    #[test]
    fn test_focus_navigation() {
        let mut manager = FocusManager::new();

        // Create widget IDs
        let id0 = make_id(&[0]);
        let id1 = make_id(&[1]);
        let id2 = make_id(&[2]);

        // Set focus chain
        manager.set_focus_chain(vec![id0.clone(), id1.clone(), id2.clone()]);

        // Should auto-focus first widget
        assert_eq!(manager.focus_id(), Some(id0.clone()));
        assert!(manager.is_focused(&id0));
        assert!(manager.is_path_focused(&[0]));

        // Focus next
        manager.focus_next();
        assert_eq!(manager.focus_id(), Some(id1.clone()));
        assert!(manager.is_path_focused(&[1]));

        manager.focus_next();
        assert_eq!(manager.focus_id(), Some(id2.clone()));
        assert!(manager.is_path_focused(&[2]));

        // Wrap around
        manager.focus_next();
        assert_eq!(manager.focus_id(), Some(id0.clone()));

        // Focus previous
        manager.focus_prev();
        assert_eq!(manager.focus_id(), Some(id2));
    }

    #[test]
    fn test_focus_within() {
        let mut manager = FocusManager::new();
        let id = make_id(&[0, 1, 2]);

        manager.set_focus_chain(vec![id]);

        assert!(manager.is_focus_within(&[]));
        assert!(manager.is_focus_within(&[0]));
        assert!(manager.is_focus_within(&[0, 1]));
        assert!(manager.is_focus_within(&[0, 1, 2]));
        assert!(!manager.is_focus_within(&[0, 1, 2, 3]));
        assert!(!manager.is_focus_within(&[1]));
    }

    #[test]
    fn test_o1_lookup_performance() {
        let mut manager = FocusManager::new();

        // Create large focus chain
        let ids: Vec<_> = (0..100).map(|i| make_id(&[i])).collect();
        manager.set_focus_chain(ids.clone());

        // Focus navigation should be O(1), not O(n)
        for _ in 0..100 {
            manager.focus_next();
        }

        // Should wrap around back to first
        assert_eq!(manager.focus_id(), Some(ids[0].clone()));
    }

    #[test]
    fn test_id_index_mapping() {
        let mut manager = FocusManager::new();

        let id0 = make_id(&[0]);
        let id1 = make_id(&[1]);
        let id2 = make_id(&[2]);

        manager.set_focus_chain(vec![id0.clone(), id1.clone(), id2.clone()]);

        // Verify index mapping is built
        assert_eq!(manager.id_to_index.len(), 3);
        assert_eq!(manager.id_to_index.get(&id0), Some(&0));
        assert_eq!(manager.id_to_index.get(&id1), Some(&1));
        assert_eq!(manager.id_to_index.get(&id2), Some(&2));
    }

    #[test]
    fn test_invalid_focus_recovery() {
        let mut manager = FocusManager::new();

        let id0 = make_id(&[0]);
        let id1 = make_id(&[1]);

        // Set initial chain with id0
        manager.set_focus_chain(vec![id0.clone()]);
        assert_eq!(manager.focus_id(), Some(id0));

        // Update chain without id0 (simulating widget removal)
        manager.set_focus_chain(vec![id1.clone()]);

        // Should auto-focus to id1
        assert_eq!(manager.focus_id(), Some(id1));
    }
}
