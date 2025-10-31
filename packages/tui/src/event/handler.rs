//! Event handler types and traits

use std::sync::Arc;

/// A boxed event handler function
pub type EventHandler<M> = Arc<dyn Fn() -> M + Send + Sync>;

/// A boxed event handler with context
pub type EventHandlerWithContext<T, M> = Arc<dyn Fn(&T) -> M + Send + Sync>;

/// Builder pattern for attaching event handlers to widgets
pub trait EventEmitter<M> {
    /// Attach an event handler that emits a message
    fn on_event<F>(self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static;
}

/// Helper to create event handlers
pub fn handler<M, F>(f: F) -> EventHandler<M>
where
    F: Fn() -> M + Send + Sync + 'static,
{
    Arc::new(f)
}

/// Helper to create event handlers with context
pub fn handler_with_context<T, M, F>(f: F) -> EventHandlerWithContext<T, M>
where
    F: Fn(&T) -> M + Send + Sync + 'static,
{
    Arc::new(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum TestMessage {
        Click,
        Input(String),
    }

    #[test]
    fn test_handler_creation() {
        let h = handler(|| TestMessage::Click);
        let msg = h();
        assert_eq!(msg, TestMessage::Click);
    }

    #[test]
    fn test_handler_with_context() {
        let h = handler_with_context(|s: &String| TestMessage::Input(s.clone()));
        let ctx = "test".to_string();
        let msg = h(&ctx);
        assert_eq!(msg, TestMessage::Input("test".to_string()));
    }
}
