//! Effect system for handling async operations
//!
//! Similar to Elm's Cmd system, Effects allow you to describe side effects
//! that should be executed outside of the pure update function.
//!
//! # Example
//!
//! ```rust,ignore
//! fn update(state: &mut State, msg: Message) -> Vec<Effect<Message>> {
//!     match msg {
//!         Message::FetchData => {
//!             vec![Effect::delay(1000, Message::DataFetched)]
//!         }
//!         Message::DataFetched => {
//!             state.loading = false;
//!             vec![]
//!         }
//!     }
//! }
//! ```

use std::sync::Arc;
use std::time::Duration;

mod runtime;
pub use runtime::EffectRuntime;

#[derive(Default)]
/// An effect that will produce a message of type M
pub enum Effect<M> {
    /// Execute after a delay
    Delay { duration: Duration, message: M },
    /// Execute a task asynchronously
    Task {
        task: Arc<dyn Fn() -> M + Send + Sync>,
    },
    /// Execute multiple effects in batch
    Batch(Vec<Effect<M>>),
    /// No operation (useful for conditional effect creation)
    #[default]
    None,
}

impl<M> Effect<M> {
    /// Create a delay effect that produces a message after a duration
    pub fn delay(duration: Duration, message: M) -> Self {
        Effect::Delay { duration, message }
    }

    /// Create a delay effect from milliseconds
    pub fn delay_ms(ms: u64, message: M) -> Self {
        Effect::Delay {
            duration: Duration::from_millis(ms),
            message,
        }
    }

    /// Create a task effect that runs asynchronously
    pub fn task<F>(task: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        Effect::Task {
            task: Arc::new(task),
        }
    }

    /// Create a batch of effects
    pub fn batch(effects: Vec<Effect<M>>) -> Self {
        Effect::Batch(effects)
    }

    /// Create a no-op effect
    pub fn none() -> Self {
        Effect::None
    }

    /// Map the message type of this effect
    pub fn map<N: 'static>(self, f: Arc<dyn Fn(M) -> N + Send + Sync>) -> Effect<N>
    where
        M: 'static,
    {
        match self {
            Effect::Delay { duration, message } => Effect::Delay {
                duration,
                message: f(message),
            },
            Effect::Task { task } => Effect::Task {
                task: Arc::new(move || f(task())),
            },
            Effect::Batch(effects) => {
                Effect::Batch(effects.into_iter().map(|e| e.map(f.clone())).collect())
            }
            Effect::None => Effect::None,
        }
    }
}

impl<M> std::fmt::Debug for Effect<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Effect::Delay { duration, .. } => {
                write!(f, "Effect::Delay({:?})", duration)
            }
            Effect::Task { .. } => write!(f, "Effect::Task(...)"),
            Effect::Batch(effects) => {
                write!(f, "Effect::Batch({} effects)", effects.len())
            }
            Effect::None => write!(f, "Effect::None"),
        }
    }
}

/// Helper macro for creating a batch of effects
#[macro_export]
macro_rules! effects {
    () => {
        vec![]
    };
    ($($effect:expr),+ $(,)?) => {
        vec![$($effect),+]
    };
}
