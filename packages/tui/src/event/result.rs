//! Event handling results

/// Result of event handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventResult {
    /// Event was handled by the widget, stop propagation
    Consumed,
    /// Event was not handled, continue propagation
    Ignored,
}

impl EventResult {
    /// Check if the event was consumed
    pub fn is_consumed(&self) -> bool {
        matches!(self, EventResult::Consumed)
    }

    /// Check if the event was ignored
    pub fn is_ignored(&self) -> bool {
        matches!(self, EventResult::Ignored)
    }
}
