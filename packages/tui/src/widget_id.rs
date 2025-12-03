//! Widget identification system
//!
//! Provides efficient widget identification using a pure ID approach:
//! - Stable integer ID derived from path (and optional key)
//! - Lightweight: only 8 bytes per ID
//! - Optional key support for maintaining identity in dynamic lists
//!
//! This design optimizes for:
//! - Fast lookups in hash maps (O(1))
//! - Fast equality comparisons (O(1))
//! - Stable IDs across widget tree rebuilds (preserves focus state)
//! - Zero-cost cloning (implements Copy)
//! - Minimal memory footprint
//!
//! Tree structure information is managed separately by FocusManager's WidgetRegistry.

use std::hash::{Hash, Hasher};

/// Pure widget identifier
///
/// # Design Philosophy
///
/// WidgetId is now a pure, lightweight identifier that:
/// - **Stable**: ID is deterministically computed from path + key
/// - **Fast**: O(1) comparison and hashing
/// - **Minimal**: Only 8 bytes
/// - **Focused**: Only does one thing - identify widgets
///
/// Tree structure, parent-child relationships, and focus management are
/// handled by FocusManager's WidgetRegistry, following separation of concerns.
///
/// # Stability Guarantee
///
/// A widget with the same path (and key, if provided) will always have the same ID,
/// even after the widget tree is rebuilt. This is critical for preserving focus state
/// in declarative UIs where the tree is recreated on each state change.
///
/// # Examples
///
/// ```
/// use tui::widget_id::WidgetId;
///
/// // Create from path - ID is stable
/// let id1 = WidgetId::from_path(&[0, 1, 2]);
/// let id2 = WidgetId::from_path(&[0, 1, 2]);
///
/// // Same path = same ID (stable across rebuilds)
/// assert_eq!(id1, id2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId {
    /// Stable numeric ID - computed from path (and optional key)
    /// Same path + key = same ID (critical for focus preservation)
    id: u64,
}

impl WidgetId {
    /// Create a new widget ID from a path
    ///
    /// Generates a stable ID based on the path. The same path will always produce
    /// the same ID, ensuring widget identity is preserved across tree rebuilds.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::widget_id::WidgetId;
    ///
    /// let id1 = WidgetId::from_path(&[0, 1]);
    /// let id2 = WidgetId::from_path(&[0, 1]);
    /// assert_eq!(id1, id2); // Stable IDs
    /// ```
    #[inline]
    pub fn from_path(path: &[usize]) -> Self {
        Self::from_path_and_key(path, None)
    }

    /// Create a new widget ID from a path and optional key
    ///
    /// The key provides additional identity information, useful for dynamic lists
    /// where path alone might not be stable (e.g., list items that can be reordered).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::widget_id::WidgetId;
    ///
    /// // For static layouts, path alone is sufficient
    /// let id1 = WidgetId::from_path_and_key(&[0], None);
    ///
    /// // For dynamic lists, use keys to maintain identity
    /// let id2 = WidgetId::from_path_and_key(&[1, 0], Some("user-123"));
    /// ```
    pub fn from_path_and_key(path: &[usize], key: Option<&str>) -> Self {
        let id = Self::compute_stable_id(path, key);
        Self { id }
    }

    /// Compute a stable ID from path and optional key
    ///
    /// Uses FxHasher (fast, non-cryptographic hash) to generate a stable ID.
    /// The ID is deterministic: same inputs always produce the same output.
    fn compute_stable_id(path: &[usize], key: Option<&str>) -> u64 {
        use rustc_hash::FxHasher;

        let mut hasher = FxHasher::default();

        // Hash the path indices
        for &index in path {
            index.hash(&mut hasher);
        }

        // Hash the key if provided
        if let Some(k) = key {
            k.hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Get the raw numeric ID value
    ///
    /// This is primarily for debugging purposes.
    /// In most cases, you should compare WidgetIds directly using == instead.
    #[inline(always)]
    pub fn id(&self) -> u64 {
        self.id
    }
}

// Default creates a root widget ID
impl Default for WidgetId {
    fn default() -> Self {
        Self::from_path(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_id_stability() {
        // Same path should produce same ID (stability guarantee)
        let id1 = WidgetId::from_path(&[0]);
        let id2 = WidgetId::from_path(&[0]);

        assert_eq!(id1, id2);
        assert_eq!(id1.id(), id2.id());
    }

    #[test]
    fn test_widget_id_with_key_stability() {
        // Same path and key should produce same ID
        let id1 = WidgetId::from_path_and_key(&[0], Some("item-1"));
        let id2 = WidgetId::from_path_and_key(&[0], Some("item-1"));

        assert_eq!(id1, id2);
        assert_eq!(id1.id(), id2.id());

        // Different keys should produce different IDs
        let id3 = WidgetId::from_path_and_key(&[0], Some("item-2"));
        assert_ne!(id1, id3);
        assert_ne!(id1.id(), id3.id());
    }

    #[test]
    fn test_widget_id_uniqueness_across_paths() {
        // Different paths should produce different IDs
        let id1 = WidgetId::from_path(&[0]);
        let id2 = WidgetId::from_path(&[1]);

        assert_ne!(id1, id2);
        assert_ne!(id1.id(), id2.id());
    }

    #[test]
    fn test_widget_id_copy() {
        let id1 = WidgetId::from_path(&[0, 1]);
        let id2 = id1; // Copy, not clone

        assert_eq!(id1, id2);
        assert_eq!(id1.id(), id2.id());
    }

    #[test]
    fn test_hash_performance() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        let id1 = WidgetId::from_path(&[0, 1, 2, 3, 4]);
        let id2 = WidgetId::from_path(&[0, 1, 2, 3, 4]);

        map.insert(id1, "value");

        // Fast O(1) lookup using ID - both IDs should be equal
        assert_eq!(map.get(&id2), Some(&"value"));
    }

    #[test]
    fn test_focus_preservation_simulation() {
        // Simulate widget tree rebuild (as happens in declarative UI)
        let build_widget_id = |path: &[usize]| WidgetId::from_path(path);

        // First render
        let focus_id_render1 = build_widget_id(&[0, 2]);

        // Simulate state change and rebuild (same path)
        let focus_id_render2 = build_widget_id(&[0, 2]);

        // IDs should be stable across rebuilds
        assert_eq!(focus_id_render1, focus_id_render2);

        // This means FocusManager can preserve focus state!
    }

    #[test]
    fn test_deterministic_id_generation() {
        // Verify ID generation is deterministic
        let path = &[1, 2, 3];
        let id1 = WidgetId::compute_stable_id(path, None);
        let id2 = WidgetId::compute_stable_id(path, None);
        assert_eq!(id1, id2);

        let id3 = WidgetId::compute_stable_id(path, Some("key"));
        let id4 = WidgetId::compute_stable_id(path, Some("key"));
        assert_eq!(id3, id4);

        // Different inputs should produce different IDs (high probability)
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_size_of_widget_id() {
        // Verify WidgetId is only 8 bytes
        assert_eq!(std::mem::size_of::<WidgetId>(), 8);
    }
}
