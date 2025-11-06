//! Event handling results

/// Result of event handling
///
/// This type allows widgets to return both event handling status and generated messages.
/// When an event is consumed, it can optionally produce one or more messages.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EventResult<M = ()> {
    /// Event was handled by the widget, stop propagation
    /// Contains any messages generated during event handling
    Consumed(Vec<M>),
    /// Event was not handled, continue propagation
    #[default]
    Ignored,
}

impl<M> EventResult<M> {
    /// Check if the event was consumed
    pub fn is_consumed(&self) -> bool {
        matches!(self, EventResult::Consumed(_))
    }

    /// Check if the event was ignored
    pub fn is_ignored(&self) -> bool {
        matches!(self, EventResult::Ignored)
    }

    /// Create a Consumed result with no messages
    pub fn consumed() -> Self {
        EventResult::Consumed(Vec::new())
    }

    /// Create a Consumed result with a single message
    pub fn consumed_with(message: M) -> Self {
        EventResult::Consumed(vec![message])
    }

    /// Create a Consumed result with multiple messages
    pub fn consumed_with_many(messages: Vec<M>) -> Self {
        EventResult::Consumed(messages)
    }

    /// Extract messages from the result
    pub fn messages(self) -> Vec<M> {
        match self {
            EventResult::Consumed(messages) => messages,
            EventResult::Ignored => Vec::new(),
        }
    }

    /// Get a reference to messages if consumed
    pub fn messages_ref(&self) -> &[M] {
        match self {
            EventResult::Consumed(messages) => messages,
            EventResult::Ignored => &[],
        }
    }

    /// Map messages to a different type
    pub fn map<N>(self, f: impl Fn(M) -> N) -> EventResult<N> {
        match self {
            EventResult::Consumed(messages) => {
                EventResult::Consumed(messages.into_iter().map(f).collect())
            }
            EventResult::Ignored => EventResult::Ignored,
        }
    }

    /// Merge two results, combining their messages if both are consumed
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (EventResult::Consumed(mut msgs1), EventResult::Consumed(msgs2)) => {
                msgs1.extend(msgs2);
                EventResult::Consumed(msgs1)
            }
            (EventResult::Consumed(msgs), EventResult::Ignored)
            | (EventResult::Ignored, EventResult::Consumed(msgs)) => EventResult::Consumed(msgs),
            (EventResult::Ignored, EventResult::Ignored) => EventResult::Ignored,
        }
    }
}
