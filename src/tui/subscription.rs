//! State-driven continuous event sources.
use crate::tui::effect::{Task, TaskContext, TaskOutcome};
use crate::tui::scope::Scope;

/// A stable event-source description, reconciled after application updates.
/// A matching key keeps its existing producer, including after it completes.
/// Remove the description to cancel it; re-adding it starts a fresh producer.
#[derive(Debug)]
pub enum Subscription<M> {
    None,
    Batch(Vec<Self>),
    Run { key: String, task: Task<M> },
    Scoped(Scope, Box<Self>),
}

impl<M> Subscription<M> {
    pub fn none() -> Self {
        Self::None
    }

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

    pub fn scope(self, scope: impl Into<Scope>) -> Self {
        Self::Scoped(scope.into(), Box::new(self))
    }

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
