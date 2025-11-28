//! Hover state management for tracking mouse enter/leave events
//!
//! This module provides a global HoverManager that maintains hover state
//! across widget tree rebuilds, enabling proper mouse enter/leave event handling.

use render::area::Area;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Widget path: index path from root container to widget
/// Example: [0, 2, 1] means root.children[0].children[2].children[1]
pub type WidgetPath = Vec<usize>;

/// Render context for tracking current widget path during rendering
///
/// This uses thread-local storage to maintain the current widget path
/// as we traverse the widget tree during rendering. Container widgets
/// (Flex, Grid) push/pop their child indices to build the path.
pub struct RenderContext;

thread_local! {
    static CURRENT_RENDER_PATH: RefCell<Vec<usize>> = RefCell::new(Vec::new());
}

impl RenderContext {
    /// Get the current widget path during rendering
    pub fn current_path() -> WidgetPath {
        CURRENT_RENDER_PATH.with(|p| p.borrow().clone())
    }

    /// Push a child index onto the current path
    ///
    /// Container widgets should call this before rendering each child.
    #[inline]
    pub fn push_index(index: usize) {
        CURRENT_RENDER_PATH.with(|p| p.borrow_mut().push(index));
    }

    /// Pop a child index from the current path
    ///
    /// Container widgets should call this after rendering each child.
    #[inline]
    pub fn pop_index() {
        CURRENT_RENDER_PATH.with(|p| p.borrow_mut().pop());
    }

    /// Clear the render path (for testing/debugging)
    #[allow(dead_code)]
    pub fn clear() {
        CURRENT_RENDER_PATH.with(|p| p.borrow_mut().clear());
    }
}

/// Global hover manager for tracking mouse enter/leave state
#[derive(Debug)]
pub struct HoverManager {
    /// Current frame's registered widgets (path -> area)
    widget_areas: HashMap<WidgetPath, Area>,

    /// Widgets currently hovered
    hovered_widgets: HashSet<WidgetPath>,

    /// Widgets that should fire enter event in current event batch
    pending_enter: HashSet<WidgetPath>,

    /// Widgets that should fire leave event in current event batch
    pending_leave: HashSet<WidgetPath>,

    /// Current mouse position
    mouse_pos: Option<(u16, u16)>,
}

impl HoverManager {
    /// Create a new hover manager
    fn new() -> Self {
        Self {
            widget_areas: HashMap::new(),
            hovered_widgets: HashSet::new(),
            pending_enter: HashSet::new(),
            pending_leave: HashSet::new(),
            mouse_pos: None,
        }
    }

    /// Get the global hover manager instance
    pub fn global() -> &'static HoverManagerHandle {
        static INSTANCE: std::sync::OnceLock<HoverManagerHandle> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(|| HoverManagerHandle {
            inner: Arc::new(RwLock::new(HoverManager::new())),
        })
    }

    /// Begin a new render frame
    #[inline]
    pub fn begin_frame(&mut self) {
        self.widget_areas.clear();
    }

    /// Register a widget's area during render phase
    #[inline]
    pub fn register_widget(&mut self, path: WidgetPath, area: Area) {
        self.widget_areas.insert(path, area);
    }

    /// End render frame - calculates initial hover state
    #[inline]
    pub fn end_frame(&mut self) {
        let new_hovered = self.calculate_hovered_widgets();
        self.hovered_widgets = new_hovered;
    }

    /// Begin processing an event batch
    #[inline]
    pub fn begin_event_batch(&mut self) {
        self.pending_enter.clear();
        self.pending_leave.clear();
    }

    /// Update mouse position and calculate hover changes
    ///
    /// Returns true if hover state changed (optimization hint)
    pub fn update_mouse_position(&mut self, x: u16, y: u16) -> bool {
        self.mouse_pos = Some((x, y));

        let new_hovered = self.calculate_hovered_widgets();

        // Calculate differences
        let has_changes = new_hovered != self.hovered_widgets;

        if has_changes {
            // Widgets entering hover
            for path in &new_hovered {
                if !self.hovered_widgets.contains(path) {
                    self.pending_enter.insert(path.clone());
                }
            }

            // Widgets leaving hover
            for path in &self.hovered_widgets {
                if !new_hovered.contains(path) {
                    self.pending_leave.insert(path.clone());
                }
            }

            self.hovered_widgets = new_hovered;
        }

        has_changes
    }

    /// Get widgets that need to receive mouse events
    ///
    /// Returns paths of widgets that either:
    /// - Are pending enter/leave events
    /// - Are currently hovered (for consistent event routing)
    pub fn get_event_targets(&self) -> Vec<&[usize]> {
        let mut targets = Vec::new();

        // Add all widgets with pending events
        for path in self.pending_enter.iter().chain(self.pending_leave.iter()) {
            targets.push(path.as_slice());
        }

        targets
    }

    /// Calculate which widgets are currently hovered based on mouse position
    #[inline]
    fn calculate_hovered_widgets(&self) -> HashSet<WidgetPath> {
        let mut hovered = HashSet::new();

        if let Some((mouse_x, mouse_y)) = self.mouse_pos {
            for (path, area) in &self.widget_areas {
                if Self::point_in_area(mouse_x, mouse_y, area) {
                    hovered.insert(path.clone());
                }
            }
        }

        hovered
    }

    /// Check if should fire mouse enter event
    #[inline]
    pub fn should_fire_enter(&mut self, path: &[usize]) -> bool {
        self.pending_enter.remove(path)
    }

    /// Check if should fire mouse leave event
    #[inline]
    pub fn should_fire_leave(&mut self, path: &[usize]) -> bool {
        self.pending_leave.remove(path)
    }

    /// Check if a point is inside an area
    #[inline]
    fn point_in_area(x: u16, y: u16, area: &Area) -> bool {
        x >= area.x() && x < area.x() + area.width() && y >= area.y() && y < area.y() + area.height()
    }
}

/// Thread-safe handle to the global HoverManager
pub struct HoverManagerHandle {
    inner: Arc<RwLock<HoverManager>>,
}

impl HoverManagerHandle {
    /// Begin a new render frame
    #[inline]
    pub fn begin_frame(&self) {
        self.inner.write().unwrap().begin_frame();
    }

    /// Register a widget's area
    #[inline]
    pub fn register_widget(&self, path: WidgetPath, area: Area) {
        self.inner.write().unwrap().register_widget(path, area);
    }

    /// End render frame
    #[inline]
    pub fn end_frame(&self) {
        self.inner.write().unwrap().end_frame();
    }

    /// Begin event batch
    #[inline]
    pub fn begin_event_batch(&self) {
        self.inner.write().unwrap().begin_event_batch();
    }

    /// Update mouse position, returns true if hover state changed
    #[inline]
    pub fn update_mouse_position(&self, x: u16, y: u16) -> bool {
        self.inner.write().unwrap().update_mouse_position(x, y)
    }

    /// Get widgets that need to receive mouse move events (for optimization)
    pub fn get_event_targets(&self) -> Vec<WidgetPath> {
        self.inner
            .read()
            .unwrap()
            .get_event_targets()
            .into_iter()
            .map(|s| s.to_vec())
            .collect()
    }

    /// Check if should fire enter event (consumes the pending state)
    #[inline]
    pub fn should_fire_enter(&self, path: &[usize]) -> bool {
        self.inner.write().unwrap().should_fire_enter(path)
    }

    /// Check if should fire leave event (consumes the pending state)
    #[inline]
    pub fn should_fire_leave(&self, path: &[usize]) -> bool {
        self.inner.write().unwrap().should_fire_leave(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hover_enter_leave() {
        let mut manager = HoverManager::new();

        manager.begin_frame();
        manager.register_widget(vec![0], Area::new((0, 0).into(), (10, 10).into()));
        manager.end_frame();

        manager.begin_event_batch();
        manager.update_mouse_position(5, 5);

        assert!(manager.should_fire_enter(&[0]));
        assert!(!manager.should_fire_leave(&[0]));

        manager.begin_event_batch();
        manager.update_mouse_position(6, 5);

        assert!(!manager.should_fire_enter(&[0]));
        assert!(!manager.should_fire_leave(&[0]));

        manager.begin_event_batch();
        manager.update_mouse_position(20, 20);

        assert!(!manager.should_fire_enter(&[0]));
        assert!(manager.should_fire_leave(&[0]));
    }
}
