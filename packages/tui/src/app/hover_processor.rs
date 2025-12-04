//! Hover event processing
//!
//! Manages hover state tracking and event generation

use crate::event::{Event, MouseEventKind};

/// Hover event processor
///
/// Coordinates with HoverManager to track mouse movement and generate hover events.
/// Batches hover state updates for efficient processing.
pub struct HoverEventProcessor;

impl HoverEventProcessor {
    /// Create a new hover event processor
    pub fn new() -> Self {
        Self
    }

    /// Begin a new frame of hover tracking
    pub fn begin_frame() {
        crate::hover::HoverManager::global().begin_frame();
    }

    /// End current frame and calculate hover states
    pub fn end_frame() {
        crate::hover::HoverManager::global().end_frame();
    }

    /// Begin a new batch of events
    ///
    /// Should be called before processing a batch of events.
    /// Clears pending hover events from last batch.
    pub fn begin_event_batch() {
        crate::hover::HoverManager::global().begin_event_batch();
    }

    /// Process a mouse event and update hover state
    ///
    /// Returns true if hover state changed (and event should be routed to widgets)
    pub fn process_mouse_event(event: &Event) -> bool {
        if let Event::Mouse(mouse_event) = event {
            if matches!(
                mouse_event.kind,
                MouseEventKind::Moved | MouseEventKind::Drag(_)
            ) {
                let has_hover_changes = crate::hover::HoverManager::global()
                    .update_mouse_position(mouse_event.column, mouse_event.row);

                // Skip routing MouseMoved if no hover state changed (optimization)
                if !has_hover_changes && matches!(mouse_event.kind, MouseEventKind::Moved) {
                    return false;
                }

                return true;
            }
        }

        // Not a mouse movement event, route normally
        true
    }

    /// Update terminal size for hover tracking
    pub fn set_terminal_size(width: u16, height: u16) {
        crate::hover::HoverManager::global().set_terminal_size(width, height);
    }
}

impl Default for HoverEventProcessor {
    fn default() -> Self {
        Self::new()
    }
}
