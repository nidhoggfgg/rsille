use std::sync::Arc;

/// Coalescing wake signal shared by producers and an event loop.
/// A wake sent before waiting is retained, so enqueue-then-wake cannot lose work.
#[derive(Clone, Debug, Default)]
pub struct WakeHandle(Arc<tokio::sync::Notify>);

impl WakeHandle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn wake(&self) {
        self.0.notify_one();
    }

    pub async fn notified(&self) {
        self.0.notified().await;
    }
}
