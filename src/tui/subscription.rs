//! State-driven continuous event sources.
//!
//! Return descriptions from [`App::subscriptions`](crate::tui::app::App::subscriptions).
//! Reconciliation uses scoped keys to retain existing producers, including
//! completed ones. Remove a description to cancel it; reintroduce its key to
//! create a fresh producer. Producers run on the blocking executor and should
//! cooperate with cancellation instead of blocking the UI thread.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use rsille::tui::{effect::TaskOutcome, subscription::Subscription};
//!
//! let ticks = Subscription::run("ticks", |ctx| {
//!     while ctx.sleep(Duration::from_secs(1)) {
//!         ctx.emit(());
//!     }
//!     TaskOutcome::Cancelled
//! });
//! // Constructing the description does not start its producer.
//! let messages = ticks.map(|()| "tick");
//! # let _ = messages;
//! ```
use crate::tui::effect::{Task, TaskContext, TaskOutcome};
use crate::tui::scope::Scope;

/// A stable event-source description, reconciled after application updates.
/// A matching key keeps its existing producer, including after it completes.
/// Remove the description to cancel it; re-adding it starts a fresh producer.
#[derive(Debug)]
pub enum Subscription<M> {
    /// Declares no continuous event sources.
    None,
    /// Combines several subscription descriptions.
    Batch(Vec<Self>),
    /// Declares a keyed producer to reconcile against the currently running sources.
    Run {
        /// Stable producer key within its resource scope.
        key: String,
        /// Producer executed when this key first becomes active.
        task: Task<M>,
    },
    /// Places the enclosed subscriptions in a nested resource namespace.
    Scoped(Scope, Box<Self>),
}

impl<M> Subscription<M> {
    /// Declares no event sources.
    pub fn none() -> Self {
        Self::None
    }

    /// Collects subscription descriptions for joint reconciliation.
    pub fn batch(sources: impl IntoIterator<Item = Self>) -> Self {
        Self::Batch(sources.into_iter().collect())
    }

    /// Runs on the configured blocking executor. Producers should cooperate
    /// with the cancellation token and emit through their task context.
    pub fn run<F>(key: impl Into<String>, producer: F) -> Self
    where
        F: Fn(TaskContext<M>) -> TaskOutcome + Send + Sync + 'static,
    {
        Self::Run {
            key: key.into(),
            task: Task::new(producer),
        }
    }

    /// Places this subscription under a nested resource scope.
    ///
    /// Match it to a mounted widget scope so resources follow that component
    /// instance's lifetime.
    pub fn scope(self, scope: impl Into<Scope>) -> Self {
        Self::Scoped(scope.into(), Box::new(self))
    }

    /// Maps producer messages and status callbacks while preserving subscription keys and scopes.
    pub fn map<N, F>(self, map: F) -> Subscription<N>
    where
        M: Send + 'static,
        N: Send + 'static,
        F: Fn(M) -> N + Send + Sync + 'static,
    {
        self.map_shared(std::sync::Arc::new(map))
    }

    fn map_shared<N>(self, map: std::sync::Arc<dyn Fn(M) -> N + Send + Sync>) -> Subscription<N>
    where
        M: Send + 'static,
        N: Send + 'static,
    {
        match self {
            Self::None => Subscription::None,
            Self::Batch(sources) => Subscription::Batch(
                sources
                    .into_iter()
                    .map(|s| s.map_shared(map.clone()))
                    .collect(),
            ),
            Self::Run { key, task } => Subscription::Run {
                key,
                task: task.map(move |m| map(m)),
            },
            Self::Scoped(scope, source) => {
                Subscription::Scoped(scope, Box::new(source.map_shared(map)))
            }
        }
    }
}
