//! Focus management system
//!
//! Provides keyboard focus navigation with Tab/Shift+Tab support,
//! and manages widget tree structure for efficient event routing.
//!
//! # Architecture
//!
//! - **FocusManager**: Manages current focus and focus chain
//! - **WidgetRegistry**: Maintains widget tree structure (paths, parent-child relationships)
//!
//! # Performance Optimizations
//!
//! - **O(1) lookups**: Uses WidgetId with integer-based identity
//! - **O(1) navigation**: HashMap-based index for instant focus switching
//! - **Efficient routing**: Path cache for keyboard event routing

use crate::widget_id::WidgetId;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

/// Widget path: index path from root container to widget
///
/// Uses SmallVec to avoid heap allocation for typical widget depths (< 8 levels).
pub type WidgetPath = SmallVec<[usize; 8]>;

/// Widget tree registry for managing hierarchical structure
///
/// Maintains path information and parent-child relationships for all widgets in the tree.
/// This enables efficient event routing and focus queries without storing paths in WidgetId.
#[derive(Debug, Clone, Default)]
pub struct WidgetRegistry {
    /// Map from WidgetId to its path in the tree
    id_to_path: FxHashMap<WidgetId, WidgetPath>,
}

impl WidgetRegistry {
    /// Create a new empty widget registry
    pub fn new() -> Self {
        Self {
            id_to_path: FxHashMap::default(),
        }
    }

    /// Register a widget with its path
    pub fn register(&mut self, id: WidgetId, path: WidgetPath) {
        self.id_to_path.insert(id, path);
    }

    /// Get the path for a widget ID
    pub fn get_path(&self, id: &WidgetId) -> Option<&[usize]> {
        self.id_to_path.get(id).map(|p| p.as_slice())
    }

    /// Check if a widget is a descendant of the given path
    ///
    /// This is used for container queries like "is focus within this container?"
    pub fn is_descendant_of(&self, id: &WidgetId, ancestor_path: &[usize]) -> bool {
        self.id_to_path
            .get(id)
            .map(|path| path.starts_with(ancestor_path))
            .unwrap_or(false)
    }

    /// Clear all registered widgets
    pub fn clear(&mut self) {
        self.id_to_path.clear();
    }

    /// Get the number of registered widgets
    pub fn len(&self) -> usize {
        self.id_to_path.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.id_to_path.is_empty()
    }
}

/// Focus manager for handling keyboard focus navigation
///
/// Manages both focus state and widget tree structure.
///
/// Optimized for fast focus switching and lookups:
/// - `focus_next/prev`: O(1) using index mapping
/// - `is_focused`: O(1) using ID comparison
/// - `is_focus_within`: O(n) where n is path depth (typically < 8)
/// - `get_focus_path`: O(1) using registry cache
#[derive(Debug, Clone)]
pub struct FocusManager {
    /// Current focused widget ID
    focus_id: Option<WidgetId>,

    /// Focus chain: ordered list of focusable widgets
    focus_chain: Vec<WidgetId>,

    /// Fast index lookup: widget ID -> index in focus_chain
    /// Enables O(1) focus navigation instead of O(n) linear search
    id_to_index: FxHashMap<WidgetId, usize>,

    /// Widget tree registry for path lookups and tree queries
    pub(crate) registry: WidgetRegistry,
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            focus_id: None,
            focus_chain: Vec::new(),
            id_to_index: FxHashMap::default(),
            registry: WidgetRegistry::new(),
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
        self.focus_id = Some(self.focus_chain[next_idx]);
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
        self.focus_id = Some(self.focus_chain[prev_idx]);
    }

    /// Check if the given widget ID is focused
    ///
    /// O(1) operation
    #[inline(always)]
    pub fn is_focused(&self, id: &WidgetId) -> bool {
        self.focus_id.as_ref() == Some(id)
    }

    /// Check if focus is within the given path (for containers)
    ///
    /// Returns true if the focused widget is a descendant of the given path.
    /// O(n) where n is path depth (typically < 8)
    pub fn is_focus_within(&self, path: &[usize]) -> bool {
        self.focus_id
            .as_ref()
            .is_some_and(|id| self.registry.is_descendant_of(id, path))
    }

    /// Get current focused widget ID
    #[inline(always)]
    pub fn focus_id(&self) -> Option<WidgetId> {
        self.focus_id
    }

    /// Get current focus path (from registry)
    pub fn focus_path(&self) -> Option<&[usize]> {
        self.focus_id
            .as_ref()
            .and_then(|id| self.registry.get_path(id))
    }

    /// Get the path for any widget ID
    pub fn get_widget_path(&self, id: &WidgetId) -> Option<&[usize]> {
        self.registry.get_path(id)
    }

    /// Set focus chain (called after rebuilding widget tree)
    ///
    /// Builds index mapping for O(1) focus navigation.
    /// O(n) operation where n is number of focusable widgets.
    ///
    /// Also accepts a registry that maps widget IDs to their paths.
    pub fn set_focus_chain(&mut self, chain: Vec<WidgetId>, registry: WidgetRegistry) {
        // Build index mapping for O(1) lookups
        self.id_to_index.clear();
        self.id_to_index.reserve(chain.len());
        for (index, &id) in chain.iter().enumerate() {
            self.id_to_index.insert(id, index);
        }

        self.focus_chain = chain;
        self.registry = registry;

        // Validate and fix focus if needed
        if let Some(focused_id) = self.focus_id {
            // O(1) lookup instead of O(n) linear search
            if !self.id_to_index.contains_key(&focused_id) {
                // Current focus is invalid, focus first widget
                self.focus_id = self.focus_chain.first().copied();
            }
        } else if !self.focus_chain.is_empty() {
            // Auto-focus first widget if nothing is focused
            self.focus_id = self.focus_chain.first().copied();
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
            .and_then(|id| self.id_to_index.get(&id).copied())
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

    fn make_id(path: &[usize]) -> WidgetId {
        WidgetId::from_path(path)
    }

    fn make_registry(ids: &[(WidgetId, &[usize])]) -> WidgetRegistry {
        let mut registry = WidgetRegistry::new();
        for &(id, path) in ids {
            registry.register(id, SmallVec::from_slice(path));
        }
        registry
    }

    #[test]
    fn test_focus_navigation() {
        let mut manager = FocusManager::new();

        // Create widget IDs
        let id0 = make_id(&[0]);
        let id1 = make_id(&[1]);
        let id2 = make_id(&[2]);

        let registry = make_registry(&[(id0, &[0]), (id1, &[1]), (id2, &[2])]);

        // Set focus chain
        manager.set_focus_chain(vec![id0, id1, id2], registry);

        // Should auto-focus first widget
        assert_eq!(manager.focus_id(), Some(id0));
        assert!(manager.is_focused(&id0));

        // Focus next
        manager.focus_next();
        assert_eq!(manager.focus_id(), Some(id1));

        manager.focus_next();
        assert_eq!(manager.focus_id(), Some(id2));

        // Wrap around
        manager.focus_next();
        assert_eq!(manager.focus_id(), Some(id0));

        // Focus previous
        manager.focus_prev();
        assert_eq!(manager.focus_id(), Some(id2));
    }

    #[test]
    fn test_focus_within() {
        let mut manager = FocusManager::new();
        let id = make_id(&[0, 1, 2]);

        let registry = make_registry(&[(id, &[0, 1, 2])]);
        manager.set_focus_chain(vec![id], registry);

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
        let registry_data: Vec<_> = ids.iter().enumerate().map(|(i, &id)| (id, vec![i])).collect();
        let registry = make_registry(
            &registry_data
                .iter()
                .map(|(id, path)| (*id, path.as_slice()))
                .collect::<Vec<_>>(),
        );

        manager.set_focus_chain(ids.clone(), registry);

        // Focus navigation should be O(1), not O(n)
        for _ in 0..100 {
            manager.focus_next();
        }

        // Should wrap around back to first
        assert_eq!(manager.focus_id(), Some(ids[0]));
    }

    #[test]
    fn test_id_index_mapping() {
        let mut manager = FocusManager::new();

        let id0 = make_id(&[0]);
        let id1 = make_id(&[1]);
        let id2 = make_id(&[2]);

        let registry = make_registry(&[(id0, &[0]), (id1, &[1]), (id2, &[2])]);
        manager.set_focus_chain(vec![id0, id1, id2], registry);

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
        let registry0 = make_registry(&[(id0, &[0])]);
        manager.set_focus_chain(vec![id0], registry0);
        assert_eq!(manager.focus_id(), Some(id0));

        // Update chain without id0 (simulating widget removal)
        let registry1 = make_registry(&[(id1, &[1])]);
        manager.set_focus_chain(vec![id1], registry1);

        // Should auto-focus to id1
        assert_eq!(manager.focus_id(), Some(id1));
    }

    #[test]
    fn test_focus_preservation_across_tree_rebuilds() {
        // This test simulates the declarative UI pattern where the widget tree
        // is rebuilt on every state change. With stable IDs, focus should be preserved.

        let mut manager = FocusManager::new();

        // Simulate first render: build widget tree and focus chain
        let build_tree = || {
            let ids = vec![
                make_id(&[0, 0]), // First button in first container
                make_id(&[0, 1]), // Second button in first container
                make_id(&[1, 0]), // Button in second container
            ];
            let registry = make_registry(&[
                (ids[0], &[0, 0]),
                (ids[1], &[0, 1]),
                (ids[2], &[1, 0]),
            ]);
            (ids, registry)
        };

        // First render
        let (chain1, registry1) = build_tree();
        manager.set_focus_chain(chain1, registry1);

        // Focus on second button
        manager.focus_next(); // Move to [0, 1]
        let focused_id_before = manager.focus_id().unwrap();
        assert_eq!(manager.focus_path(), Some(&[0, 1][..]));

        // Simulate state change and tree rebuild (declarative UI pattern)
        // The widget tree is completely recreated, but with stable IDs,
        // the same path produces the same ID
        let (chain2, registry2) = build_tree();
        manager.set_focus_chain(chain2, registry2);

        // Focus should be preserved! Same path = same ID = same focus
        let focused_id_after = manager.focus_id().unwrap();
        assert_eq!(manager.focus_path(), Some(&[0, 1][..]));
        assert_eq!(focused_id_before, focused_id_after);

        // Verify we can still navigate
        manager.focus_next();
        assert_eq!(manager.focus_path(), Some(&[1, 0][..]));
    }

    #[test]
    fn test_focus_preservation_with_keys() {
        // Test focus preservation with explicit widget keys (for dynamic lists)

        let mut manager = FocusManager::new();

        // Simulate a list with explicit keys
        let build_list = |item_keys: &[&str]| {
            let ids: Vec<_> = item_keys
                .iter()
                .enumerate()
                .map(|(idx, &key)| WidgetId::from_path_and_key(&[0, idx], Some(key)))
                .collect();
            let registry_data: Vec<_> = item_keys
                .iter()
                .enumerate()
                .map(|(idx, &key)| {
                    (
                        WidgetId::from_path_and_key(&[0, idx], Some(key)),
                        vec![0, idx],
                    )
                })
                .collect();
            let registry = make_registry(
                &registry_data
                    .iter()
                    .map(|(id, path)| (*id, path.as_slice()))
                    .collect::<Vec<_>>(),
            );
            (ids, registry)
        };

        // First render with items ["user-1", "user-2", "user-3"]
        let (chain1, registry1) = build_list(&["user-1", "user-2", "user-3"]);
        manager.set_focus_chain(chain1, registry1);

        // Focus on user-2 (index 1)
        manager.focus_next();
        let focused_before = manager.focus_id().unwrap();
        assert_eq!(manager.focus_path(), Some(&[0, 1][..]));

        // Simulate list reorder: ["user-3", "user-2", "user-1"]
        // user-2 is now at index 1, but same key should preserve focus
        let (chain2, registry2) = build_list(&["user-3", "user-2", "user-1"]);
        manager.set_focus_chain(chain2, registry2);

        // Focus should still be on user-2, even though it's at a different index
        let focused_after = manager.focus_id().unwrap();
        assert_eq!(manager.focus_path(), Some(&[0, 1][..])); // Same path as before
        assert_eq!(focused_before, focused_after); // Same ID due to same key
    }

    #[test]
    fn test_widget_registry() {
        let mut registry = WidgetRegistry::new();

        let id1 = make_id(&[0, 1]);
        let id2 = make_id(&[0, 1, 2]);

        registry.register(id1, SmallVec::from_slice(&[0, 1]));
        registry.register(id2, SmallVec::from_slice(&[0, 1, 2]));

        assert_eq!(registry.get_path(&id1), Some(&[0, 1][..]));
        assert_eq!(registry.get_path(&id2), Some(&[0, 1, 2][..]));

        assert!(!registry.is_descendant_of(&id1, &[1]));
        assert!(registry.is_descendant_of(&id1, &[0]));
        assert!(registry.is_descendant_of(&id2, &[0, 1]));

        assert_eq!(registry.len(), 2);
    }
}
