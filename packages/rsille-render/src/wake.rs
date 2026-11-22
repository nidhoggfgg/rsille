//! Shared notifications for event-driven application updates.

use std::sync::Arc;

/// Coalescing wake signal shared by producers and an event loop.
/// A wake sent before waiting is retained, so enqueue-then-wake cannot lose work.
#[derive(Clone, Debug, Default)]
pub struct WakeHandle(Arc<tokio::sync::Notify>);

impl WakeHandle {
    /// Creates a wake signal with no pending notification.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wakes one waiter, or retains one notification if nobody is waiting.
    ///
    /// Enqueue work before calling this method. Multiple pending notifications
    /// coalesce, so consumers must drain their work queue after waking.
    pub fn wake(&self) {
        self.0.notify_one();
    }

    /// Waits for and consumes a notification shared by all clones of this handle.
    pub async fn notified(&self) {
        self.0.notified().await;
    }
}
