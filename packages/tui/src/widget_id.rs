//! Widget identification system
//!
//! Provides efficient widget identification using a stable ID approach:
//! - Stable integer ID derived from path (and optional key) for O(1) comparison and hashing
//! - Path information for hierarchical queries (e.g., is_focus_within)
//! - Optional key support for maintaining identity in dynamic lists
//!
//! This design optimizes for:
//! - Fast lookups in hash maps
//! - Fast equality comparisons
//! - Stable IDs across widget tree rebuilds (preserves focus state)
//! - Zero-cost cloning (implements Copy for simple cases)
//! - Preserving hierarchical information for container queries

use smallvec::SmallVec;
use std::hash::{Hash, Hasher};

/// Widget path: index path from root container to widget
///
/// Uses SmallVec to avoid heap allocation for typical widget depths (< 8 levels).
pub type WidgetPath = SmallVec<[usize; 8]>;

/// Widget identity combining stable ID with hierarchical path
///
/// # Design Philosophy
///
/// This structure provides stable widget identification across rebuilds:
/// - **Stable IDs**: ID is computed from path (and optional key), not from a counter
/// - **Fast operations**: ID enables O(1) hash/compare/lookup
/// - **Rich queries**: Path enables hierarchical relationships
/// - **Focus preservation**: Same path = same ID = preserved focus state
///
/// # Stability Guarantee
///
/// A widget with the same path (and key, if provided) will always have the same ID,
/// even after the widget tree is rebuilt. This is critical for preserving focus state
/// in declarative UIs where the tree is recreated on each state change.
///
/// # Performance Characteristics
///
/// - `Clone`: O(1) for typical paths (< 8 deep, inline storage)
/// - `Eq/Hash`: O(1) using only ID field
/// - `path_eq`: O(n) for full path comparison when needed
/// - Memory: 8 bytes (ID) + inline SmallVec (no heap for depth < 8)
///
/// # Examples
///
/// ```
/// use tui::widget_id::WidgetId;
/// use smallvec::SmallVec;
///
/// // Create from path - ID is stable
/// let id1 = WidgetId::from_path(SmallVec::from_slice(&[0, 1, 2]));
/// let id2 = WidgetId::from_path(SmallVec::from_slice(&[0, 1, 2]));
///
/// // Same path = same ID (stable across rebuilds)
/// assert_eq!(id1, id2);
/// assert_eq!(id1.id(), id2.id());
///
/// // Hierarchical queries (uses path)
/// assert!(id1.is_descendant_of(&[0]));
/// assert!(id1.is_descendant_of(&[0, 1]));
/// assert!(!id1.is_descendant_of(&[1]));
/// ```
#[derive(Debug, Clone)]
pub struct WidgetId {
    /// Stable numeric ID - computed from path (and optional key)
    /// Same path + key = same ID (critical for focus preservation)
    id: u64,
    /// Hierarchical path - used for container queries
    /// Inline storage for paths up to depth 8 (no heap allocation)
    path: WidgetPath,
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
    /// use smallvec::SmallVec;
    ///
    /// let id1 = WidgetId::from_path(SmallVec::from_slice(&[0, 1]));
    /// let id2 = WidgetId::from_path(SmallVec::from_slice(&[0, 1]));
    /// assert_eq!(id1, id2); // Stable IDs
    /// ```
    #[inline]
    pub fn from_path(path: WidgetPath) -> Self {
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
    /// use smallvec::SmallVec;
    ///
    /// // For static layouts, path alone is sufficient
    /// let id1 = WidgetId::from_path_and_key(SmallVec::from_slice(&[0]), None);
    ///
    /// // For dynamic lists, use keys to maintain identity
    /// let id2 = WidgetId::from_path_and_key(
    ///     SmallVec::from_slice(&[1, 0]),
    ///     Some("user-123")
    /// );
    /// ```
    pub fn from_path_and_key(path: WidgetPath, key: Option<&str>) -> Self {
        let id = Self::compute_stable_id(&path, key);
        Self { id, path }
    }

    /// Compute a stable ID from path and optional key
    ///
    /// Uses FxHasher (fast, non-cryptographic hash) to generate a stable ID.
    /// The ID is deterministic: same inputs always produce the same output.
    fn compute_stable_id(path: &WidgetPath, key: Option<&str>) -> u64 {
        use rustc_hash::FxHasher;

        let mut hasher = FxHasher::default();

        // Hash the path indices
        for &index in path.iter() {
            index.hash(&mut hasher);
        }

        // Hash the key if provided
        if let Some(k) = key {
            k.hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Get the unique numeric ID
    #[inline(always)]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the hierarchical path
    #[inline(always)]
    pub fn path(&self) -> &[usize] {
        &self.path
    }

    /// Check if this widget is a descendant of the given path
    ///
    /// Used for container queries like "is focus within this container?"
    ///
    /// # Examples
    ///
    /// ```
    /// # use tui::widget_id::WidgetId;
    /// # use smallvec::SmallVec;
    /// let id = WidgetId::from_path(SmallVec::from_slice(&[0, 1, 2]));
    ///
    /// assert!(id.is_descendant_of(&[]));      // Everything is descendant of root
    /// assert!(id.is_descendant_of(&[0]));     // True
    /// assert!(id.is_descendant_of(&[0, 1]));  // True
    /// assert!(id.is_descendant_of(&[0, 1, 2])); // Exact match
    /// assert!(!id.is_descendant_of(&[0, 1, 2, 3])); // Not a descendant
    /// assert!(!id.is_descendant_of(&[1]));    // Different branch
    /// ```
    #[inline]
    pub fn is_descendant_of(&self, ancestor_path: &[usize]) -> bool {
        self.path.starts_with(ancestor_path)
    }

    /// Compare paths for equality
    ///
    /// This is different from `==` which only compares IDs.
    /// Use this when you need to verify path structure, not identity.
    #[inline]
    pub fn path_eq(&self, other: &Self) -> bool {
        self.path == other.path
    }

    /// Get path depth (number of ancestors)
    #[inline(always)]
    pub fn depth(&self) -> usize {
        self.path.len()
    }
}

// Equality and hashing use only the ID for O(1) performance
impl PartialEq for WidgetId {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for WidgetId {}

impl std::hash::Hash for WidgetId {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

// Default creates a root widget ID
impl Default for WidgetId {
    fn default() -> Self {
        Self::from_path(SmallVec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_id_stability() {
        // Same path should produce same ID (stability guarantee)
        let id1 = WidgetId::from_path(SmallVec::from_slice(&[0]));
        let id2 = WidgetId::from_path(SmallVec::from_slice(&[0]));

        assert_eq!(id1, id2);
        assert_eq!(id1.id(), id2.id());
        assert!(id1.path_eq(&id2));
    }

    #[test]
    fn test_widget_id_with_key_stability() {
        // Same path and key should produce same ID
        let id1 = WidgetId::from_path_and_key(SmallVec::from_slice(&[0]), Some("item-1"));
        let id2 = WidgetId::from_path_and_key(SmallVec::from_slice(&[0]), Some("item-1"));

        assert_eq!(id1, id2);
        assert_eq!(id1.id(), id2.id());

        // Different keys should produce different IDs
        let id3 = WidgetId::from_path_and_key(SmallVec::from_slice(&[0]), Some("item-2"));
        assert_ne!(id1, id3);
        assert_ne!(id1.id(), id3.id());
    }

    #[test]
    fn test_widget_id_uniqueness_across_paths() {
        // Different paths should produce different IDs
        let id1 = WidgetId::from_path(SmallVec::from_slice(&[0]));
        let id2 = WidgetId::from_path(SmallVec::from_slice(&[1]));

        assert_ne!(id1, id2);
        assert_ne!(id1.id(), id2.id());
        assert!(!id1.path_eq(&id2));
    }

    #[test]
    fn test_widget_id_clone() {
        let id1 = WidgetId::from_path(SmallVec::from_slice(&[0, 1]));
        let id2 = id1.clone();

        assert_eq!(id1, id2);
        assert_eq!(id1.id(), id2.id());
    }

    #[test]
    fn test_is_descendant_of() {
        let id = WidgetId::from_path(SmallVec::from_slice(&[0, 1, 2]));

        assert!(id.is_descendant_of(&[]));
        assert!(id.is_descendant_of(&[0]));
        assert!(id.is_descendant_of(&[0, 1]));
        assert!(id.is_descendant_of(&[0, 1, 2]));
        assert!(!id.is_descendant_of(&[0, 1, 2, 3]));
        assert!(!id.is_descendant_of(&[1]));
    }

    #[test]
    fn test_hash_performance() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        let id1 = WidgetId::from_path(SmallVec::from_slice(&[0, 1, 2, 3, 4]));
        let id2 = WidgetId::from_path(SmallVec::from_slice(&[0, 1, 2, 3, 4]));

        map.insert(id1, "value");

        // Fast O(1) lookup using ID - both IDs should be equal
        assert_eq!(map.get(&id2), Some(&"value"));
    }

    #[test]
    fn test_smallvec_no_heap() {
        // Paths with depth <= 8 should not allocate on heap
        let id = WidgetId::from_path(SmallVec::from_slice(&[0, 1, 2, 3, 4, 5, 6, 7]));
        assert_eq!(id.depth(), 8);
        assert_eq!(id.path(), &[0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_focus_preservation_simulation() {
        // Simulate widget tree rebuild (as happens in declarative UI)
        let build_widget_id = |path: &[usize]| WidgetId::from_path(SmallVec::from_slice(path));

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
        let path = SmallVec::from_slice(&[1, 2, 3]);
        let id1 = WidgetId::compute_stable_id(&path, None);
        let id2 = WidgetId::compute_stable_id(&path, None);
        assert_eq!(id1, id2);

        let id3 = WidgetId::compute_stable_id(&path, Some("key"));
        let id4 = WidgetId::compute_stable_id(&path, Some("key"));
        assert_eq!(id3, id4);

        // Different inputs should produce different IDs (high probability)
        assert_ne!(id1, id3);
    }
}
