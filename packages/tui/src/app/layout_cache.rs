//! Layout caching system
//!
//! Manages cached widget tree to avoid rebuilding on every frame

use crate::layout::Layout;

/// Layout cache manager
///
/// Caches the widget tree to preserve internal widget state between frames
/// (like button pressed state, text input cursor position, etc.)
///
/// Only rebuilds when:
/// - State changes (via messages)
/// - Terminal resizes
/// - Explicit invalidation
pub struct LayoutCache<M> {
    /// Cached widget tree
    cached_layout: Option<Box<dyn Layout<M>>>,
    /// Flag indicating state has changed and tree needs rebuild
    state_changed: bool,
}

impl<M> LayoutCache<M> {
    /// Create a new layout cache
    pub fn new() -> Self {
        Self {
            cached_layout: None,
            state_changed: true, // Start with state_changed=true to build initial tree
        }
    }

    /// Mark cache as invalid (will rebuild on next access)
    pub fn invalidate(&mut self) {
        self.state_changed = true;
    }

    /// Clear the cache completely
    pub fn clear(&mut self) {
        self.cached_layout = None;
        self.state_changed = true;
    }

    /// Get cached layout, rebuilding if necessary
    pub fn get_or_rebuild<F>(&mut self, builder: F) -> &mut Box<dyn Layout<M>>
    where
        F: FnOnce() -> Box<dyn Layout<M>>,
    {
        if self.state_changed || self.cached_layout.is_none() {
            self.cached_layout = Some(builder());
            self.state_changed = false;
        }

        self.cached_layout
            .as_mut()
            .expect("Layout should be cached after rebuild")
    }

    /// Get cached layout without rebuilding (returns None if not cached)
    pub fn get(&self) -> Option<&Box<dyn Layout<M>>> {
        self.cached_layout.as_ref()
    }

    /// Get mutable cached layout without rebuilding (returns None if not cached)
    pub fn get_mut(&mut self) -> Option<&mut Box<dyn Layout<M>>> {
        self.cached_layout.as_mut()
    }

    /// Check if cache is valid
    pub fn is_valid(&self) -> bool {
        self.cached_layout.is_some() && !self.state_changed
    }

    /// Check if state has changed
    pub fn needs_rebuild(&self) -> bool {
        self.state_changed || self.cached_layout.is_none()
    }

    /// Set the cached layout directly
    pub fn set(&mut self, layout: Box<dyn Layout<M>>) {
        self.cached_layout = Some(layout);
        self.state_changed = false;
    }
}

impl<M> Default for LayoutCache<M> {
    fn default() -> Self {
        Self::new()
    }
}
