//! Overlay rendering system for floating widgets
//!
//! This module provides a mechanism for widgets to render content that floats
//! above other widgets, similar to dropdowns and modals in web frameworks.

use render::area::Area;
use render::chunk::Chunk;
use std::sync::{Mutex, OnceLock};

/// A closure that renders overlay content
type OverlayRenderer = Box<dyn FnOnce(&mut Chunk) + Send>;

/// Information about an overlay to be rendered
pub struct OverlayInfo {
    /// The area where the overlay should be rendered
    pub area: Area,
    /// The rendering function
    pub renderer: OverlayRenderer,
    /// Z-index for layering (higher values render on top)
    pub z_index: i32,
}

impl OverlayInfo {
    /// Create a new overlay info
    pub fn new<F>(area: Area, z_index: i32, renderer: F) -> Self
    where
        F: FnOnce(&mut Chunk) + Send + 'static,
    {
        Self {
            area,
            z_index,
            renderer: Box::new(renderer),
        }
    }
}

/// Global overlay manager
pub struct OverlayManager {
    overlays: Mutex<Vec<OverlayInfo>>,
}

impl OverlayManager {
    /// Create a new overlay manager
    fn new() -> Self {
        Self {
            overlays: Mutex::new(Vec::new()),
        }
    }

    /// Get the global overlay manager instance
    pub fn global() -> &'static OverlayManager {
        static INSTANCE: OnceLock<OverlayManager> = OnceLock::new();
        INSTANCE.get_or_init(OverlayManager::new)
    }

    /// Register an overlay to be rendered
    pub fn add_overlay(&self, overlay: OverlayInfo) {
        let mut overlays = self.overlays.lock().unwrap();
        overlays.push(overlay);
    }

    /// Take all overlays and clear the queue
    ///
    /// Overlays are returned sorted by z-index (lowest first)
    pub fn take_overlays(&self) -> Vec<OverlayInfo> {
        let mut overlays = self.overlays.lock().unwrap();
        let mut taken = Vec::new();
        std::mem::swap(&mut *overlays, &mut taken);

        // Sort by z-index
        taken.sort_by_key(|o| o.z_index);
        taken
    }

    /// Clear all overlays without rendering them
    pub fn clear(&self) {
        let mut overlays = self.overlays.lock().unwrap();
        overlays.clear();
    }
}
