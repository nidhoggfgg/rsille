//! Widget identification system
//!
//! Provides efficient widget identification using a hybrid approach:
//! - Unique integer ID for O(1) comparison and hashing
//! - Path information for hierarchical queries (e.g., is_focus_within)
//!
//! This design optimizes for:
//! - Fast lookups in hash maps
//! - Fast equality comparisons
//! - Zero-cost cloning (implements Copy)
//! - Preserving hierarchical information for container queries

use smallvec::SmallVec;
use std::sync::atomic::{AtomicU64, Ordering};

/// Widget path: index path from root container to widget
///
/// Uses SmallVec to avoid heap allocation for typical widget depths (< 8 levels).
pub type WidgetPath = SmallVec<[usize; 8]>;

/// Global widget ID counter
static WIDGET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Widget identity combining efficient ID with hierarchical path
///
/// # Design Philosophy
///
/// This structure provides the best of both worlds:
/// - **Fast operations**: ID enables O(1) hash/compare/lookup
/// - **Rich queries**: Path enables hierarchical relationships
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
/// // Create from path during render
/// let id = WidgetId::from_path(SmallVec::from_slice(&[0, 1, 2]));
///
/// // Fast comparison (uses ID)
/// let id2 = id.clone();
/// assert_eq!(id, id2);
///
/// // Hierarchical queries (uses path)
/// assert!(id.is_descendant_of(&[0]));
/// assert!(id.is_descendant_of(&[0, 1]));
/// assert!(!id.is_descendant_of(&[1]));
/// ```
#[derive(Debug, Clone)]
pub struct WidgetId {
    /// Unique numeric ID - primary key for fast operations
    id: u64,
    /// Hierarchical path - used for container queries
    /// Inline storage for paths up to depth 8 (no heap allocation)
    path: WidgetPath,
}

impl WidgetId {
    /// Create a new widget ID from a path
    ///
    /// Allocates a globally unique ID and associates it with the path.
    /// IDs are monotonically increasing and unique within a program run.
    #[inline]
    pub fn from_path(path: WidgetPath) -> Self {
        Self {
            id: WIDGET_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            path,
        }
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
    fn test_widget_id_uniqueness() {
        let id1 = WidgetId::from_path(SmallVec::from_slice(&[0]));
        let id2 = WidgetId::from_path(SmallVec::from_slice(&[0]));

        // Same path but different IDs
        assert_ne!(id1, id2);
        assert_ne!(id1.id(), id2.id());
        assert!(id1.path_eq(&id2));
    }

    #[test]
    fn test_widget_id_copy() {
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
        let id2 = id1.clone();

        map.insert(id1, "value");

        // Fast O(1) lookup using ID
        assert_eq!(map.get(&id2), Some(&"value"));
    }

    #[test]
    fn test_smallvec_no_heap() {
        // Paths with depth <= 8 should not allocate on heap
        let id = WidgetId::from_path(SmallVec::from_slice(&[0, 1, 2, 3, 4, 5, 6, 7]));
        assert_eq!(id.depth(), 8);
        assert_eq!(id.path(), &[0, 1, 2, 3, 4, 5, 6, 7]);
    }
}
