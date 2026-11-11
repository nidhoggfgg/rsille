//! Update effects, background tasks, typed requests, and cooperative cancellation.
//!
//! Queue work through [`UpdateCtx`] or compose [`Effect`] values. [`Task`] can
//! emit multiple messages; [`Request`] adds typed results and lifecycle events
//! suitable for [`RequestState`]. Both use the runtime's configured blocking
//! executor. Retries invoke the same runner again, so captured inputs must remain
//! suitable for repeated execution.
//!
//! Task timeouts and delayed messages use the runtime clock. Cancellation prevents
//! late result delivery and sets a shared token, but cannot forcibly stop worker
//! code. Check the context token or use its cancellation-aware sleep. Keyed work
//! can be attached to mounted component namespaces with [`Effect::scope`].
//!
//! # Examples
//!
//! A manual clock makes delayed updates testable without sleeping:
//!
//! ```
//! use std::time::Duration;
//! use rsille::tui::{app::App, runtime::ManualClock, widgets::label};
//!
//! enum Message { Start, Ready }
//! let clock = ManualClock::default();
//! let mut runtime = App::new(false)
//!     .with_clock(clock.clone())
//!     .update(|ready, message, ctx| match message {
//!         Message::Start => ctx.after(Duration::from_millis(50), Message::Ready),
//!         Message::Ready => *ready = true,
//!     })
//!     .view(|ready| label(if *ready { "Ready" } else { "Waiting" }))
//!     .build()
//!     .size(12, 1);
//!
//! runtime.handle().send(Message::Start).unwrap();
//! runtime.step(&[]).unwrap();
//! assert!(!runtime.state());
//! clock.advance(Duration::from_millis(50));
//! runtime.step(&[]).unwrap();
//! assert!(*runtime.state());
//! ```

use crate::tui::scope::{ResourceKey, Scope};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc::Sender, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Runtime-assigned identity of a spawned task.
///
/// IDs identify tasks within a runtime; retries keep the same task ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub(crate) u64);

impl TaskId {
    /// Returns the underlying numeric identifier.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Monotonically allocated identity of a request converted into a task.
///
/// [`RequestState`] uses ordering between IDs to reject stale request events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestId(u64);

impl RequestId {
    fn next() -> Self {
        static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Returns the underlying numeric identifier.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Runtime lifecycle state of a background task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Registered and waiting to start its attempt.
    Queued,
    /// An attempt has been submitted to the executor.
    Running,
    /// Waiting for the retry delay before another attempt.
    RetryScheduled,
    /// Cancellation has been requested.
    Cancelling,
    /// The runner completed successfully.
    Completed,
    /// The task was cancelled; further results are rejected.
    Cancelled,
    /// The attempt exceeded its configured timeout.
    TimedOut,
    /// The runner requested a retry after exhausting its retry budget.
    Failed,
    /// The runner panicked during an attempt.
    Panicked,
}

impl TaskState {
    /// Returns whether no further attempt is scheduled for this lifecycle state.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskState::Completed
                | TaskState::Cancelled
                | TaskState::TimedOut
                | TaskState::Failed
                | TaskState::Panicked
        )
    }
}

/// Snapshot of task identity, lifecycle, attempt count, and timing.
#[derive(Debug, Clone)]
pub struct TaskStatus {
    /// Identity shared by all attempts of this task.
    pub id: TaskId,
    /// Optional replacement/cancellation key within the owning scope.
    pub key: Option<String>,
    /// Optional user-facing task description.
    pub label: Option<String>,
    /// Namespace of the owning component instance.
    pub scope: Scope,
    /// Current runtime lifecycle state.
    pub state: TaskState,
    /// One-based attempt number; the initial attempt is 1.
    pub attempt: u32,
    /// Maximum total attempts, including the initial attempt.
    pub max_attempts: u32,
    /// Runtime-clock instant when the task was registered.
    pub created_at: Instant,
    /// Runtime-clock instant of the latest status change.
    pub updated_at: Instant,
}

impl TaskStatus {
    pub(crate) fn new(
        id: TaskId,
        key: Option<String>,
        label: Option<String>,
        state: TaskState,
        attempt: u32,
        max_attempts: u32,
        created_at: Instant,
    ) -> Self {
        Self {
            id,
            key,
            label,
            scope: Scope::root(),
            state,
            attempt,
            max_attempts,
            created_at,
            updated_at: created_at,
        }
    }

    /// Returns whether the task is queued, running, waiting to retry, or cancelling.
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            TaskState::Queued
                | TaskState::Running
                | TaskState::RetryScheduled
                | TaskState::Cancelling
        )
    }

    /// Returns whether an attempt is in the running state.
    pub fn is_running(&self) -> bool {
        self.state == TaskState::Running
    }

    /// Returns whether the task is queued or waiting for its retry delay.
    pub fn is_pending(&self) -> bool {
        matches!(self.state, TaskState::Queued | TaskState::RetryScheduled)
    }
}

/// Outcome returned by one invocation of a task runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskOutcome {
    /// Completes the task successfully (default).
    #[default]
    Complete,
    /// Requests another attempt; exhaustion changes task state to failed.
    Retry,
    /// Finishes the task as cancelled.
    Cancelled,
}

/// Maximum retry count and fixed delay between attempts.
///
/// Retries exclude the initial attempt. The default permits no retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    max_retries: u32,
    delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::none()
    }
}

impl RetryPolicy {
    /// Disables retries and sets the retry delay to zero.
    pub const fn none() -> Self {
        Self {
            max_retries: 0,
            delay: Duration::ZERO,
        }
    }

    /// Allows `max_retries` additional attempts with no delay.
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            delay: Duration::ZERO,
        }
    }

    /// Allows `max_retries` additional attempts separated by a fixed delay.
    pub fn fixed(max_retries: u32, delay: Duration) -> Self {
        Self { max_retries, delay }
    }

    /// Sets the fixed delay between attempts.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Returns the number of permitted retries, excluding the first attempt.
    pub fn max_retries(self) -> u32 {
        self.max_retries
    }

    /// Returns the delay between attempts.
    pub fn delay(self) -> Duration {
        self.delay
    }

    /// Returns retries plus the initial attempt, saturating at `u32::MAX`.
    pub fn max_attempts(self) -> u32 {
        self.max_retries.saturating_add(1)
    }

    pub(crate) fn can_retry(self, attempt: u32) -> bool {
        attempt <= self.max_retries
    }
}

/// A clonable, cooperative cancellation flag shared across threads.
///
/// Cancelling any clone sets the same flag. It does not interrupt a running
/// thread; task code must check the flag or use a cancellation-aware sleep.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates an unset cancellation flag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Permanently sets the shared cancellation flag without waiting for a worker.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Returns whether this token or any of its clones has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// A description of work to perform after an application update.
///
/// Construct effects directly or enqueue them with [`UpdateCtx`]. Keyed tasks
/// and debounces are isolated by [`Scope`]; a batch preserves declaration order
/// without waiting for spawned tasks to finish.
#[derive(Debug)]
pub enum Effect<M> {
    /// Performs no work.
    None,
    /// Request application exit after the current update.
    Exit,
    /// Queues an application message.
    Message(M),
    /// Presents and commits the current inline frame before delivering this message.
    CommitFrame(M),
    /// Applies effects in declaration order without awaiting their completion.
    Batch(Vec<Effect<M>>),
    /// Submits a task to the configured executor.
    Spawn(Task<M>),
    /// Cancels a task by runtime identity.
    CancelTask(TaskId),
    /// Cancels the task with this key in the current resource scope.
    CancelTaskKey(String),
    /// Queues a message once the given delay has elapsed.
    After(Duration, M),
    /// Replaces a delayed message with the same key in the current resource scope.
    Debounce {
        /// Debounce key, isolated within the current resource scope.
        key: String,
        /// Delay from scheduling until the message becomes eligible for delivery.
        duration: Duration,
        /// Application message delivered if the debounce is not replaced or cancelled.
        message: M,
    },
    /// Removes the pending debounce with this scoped key.
    CancelDebounce(String),
    /// Applies the enclosed effect under a nested resource scope.
    Scoped(Scope, Box<Effect<M>>),
}

impl<M> Effect<M> {
    /// Request exit after the current update, including from mapped child effects.
    pub fn exit() -> Self {
        Self::Exit
    }

    /// Creates an effect that performs no work.
    pub fn none() -> Self {
        Self::None
    }

    /// Queues a message for a subsequent application update.
    pub fn message(message: M) -> Self {
        Self::Message(message)
    }

    /// Creates a hard presentation boundary. See [`UpdateCtx::commit_frame`].
    pub fn commit_frame(after_message: M) -> Self {
        Self::CommitFrame(after_message)
    }

    /// Collects effects in declaration order; spawned tasks are not awaited.
    pub fn batch(effects: impl IntoIterator<Item = Effect<M>>) -> Self {
        Self::Batch(effects.into_iter().collect())
    }

    /// Schedules a task on the configured executor.
    pub fn spawn(task: Task<M>) -> Self {
        Self::Spawn(task)
    }

    /// Requests cancellation by task ID.
    ///
    /// The root scope may cancel any task; a scoped effect can cancel only a task
    /// owned by that scope.
    pub fn cancel_task(task_id: TaskId) -> Self {
        Self::CancelTask(task_id)
    }

    /// Cancels the task with this key in the effect's resource scope.
    pub fn cancel_task_key(key: impl Into<String>) -> Self {
        Self::CancelTaskKey(key.into())
    }

    /// Schedules a message after a delay measured by the runtime clock.
    pub fn after(duration: Duration, message: M) -> Self {
        Self::After(duration, message)
    }

    /// Schedules a delayed message, replacing the timer and payload for the same scoped key.
    pub fn debounce(key: impl Into<String>, duration: Duration, message: M) -> Self {
        Self::Debounce {
            key: key.into(),
            duration,
            message,
        }
    }

    /// Cancels a pending debounce in the effect's resource scope.
    pub fn cancel_debounce(key: impl Into<String>) -> Self {
        Self::CancelDebounce(key.into())
    }
    /// Attach resources and messages to a mounted widget scope.
    pub fn scope(self, scope: impl Into<Scope>) -> Self {
        Self::Scoped(scope.into(), Box::new(self))
    }

    /// Maps every message payload, including task and request callbacks, to a new message type.
    ///
    /// Cancellation, exit, scope, and scheduling metadata are preserved.
    pub fn map<N, F>(self, map: F) -> Effect<N>
    where
        M: Send + 'static,
        N: Send + 'static,
        F: Fn(M) -> N + Send + Sync + 'static,
    {
        self.map_shared(Arc::new(map))
    }

    fn map_shared<N>(self, map: Arc<dyn Fn(M) -> N + Send + Sync>) -> Effect<N>
    where
        M: Send + 'static,
        N: Send + 'static,
    {
        match self {
            Self::None => Effect::None,
            Self::Exit => Effect::Exit,
            Self::Message(m) => Effect::Message(map(m)),
            Self::CommitFrame(m) => Effect::CommitFrame(map(m)),
            Self::Batch(effects) => Effect::Batch(
                effects
                    .into_iter()
                    .map(|e| e.map_shared(map.clone()))
                    .collect(),
            ),
            Self::Spawn(task) => Effect::Spawn(task.map(move |m| map(m))),
            Self::CancelTask(id) => Effect::CancelTask(id),
            Self::CancelTaskKey(key) => Effect::CancelTaskKey(key),
            Self::After(duration, m) => Effect::After(duration, map(m)),
            Self::Debounce {
                key,
                duration,
                message,
            } => Effect::Debounce {
                key,
                duration,
                message: map(message),
            },
            Self::CancelDebounce(key) => Effect::CancelDebounce(key),
            Self::Scoped(scope, effect) => Effect::Scoped(scope, Box::new(effect.map_shared(map))),
        }
    }
}

type TaskRunner<M> = Arc<dyn Fn(TaskContext<M>) -> TaskOutcome + Send + Sync + 'static>;
pub(crate) type StatusHandler<M> = Arc<dyn Fn(TaskStatus) -> Option<M> + Send + Sync + 'static>;

/// A repeatable blocking job that can emit application messages.
///
/// The runtime submits each attempt to its configured executor. Cancellation
/// and timeout stop accepting results but cannot forcibly stop blocking code;
/// cooperate through [`TaskContext::is_cancelled`] or [`TaskContext::sleep`].
pub struct Task<M> {
    pub(crate) key: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) retry: RetryPolicy,
    pub(crate) runner: TaskRunner<M>,
    pub(crate) status_handler: Option<StatusHandler<M>>,
}

impl<M> Task<M> {
    /// Creates an unkeyed task with no timeout, retries, or status callback.
    pub fn new<F>(runner: F) -> Self
    where
        F: Fn(TaskContext<M>) -> TaskOutcome + Send + Sync + 'static,
    {
        Self {
            key: None,
            label: None,
            timeout: None,
            retry: RetryPolicy::none(),
            runner: Arc::new(runner),
            status_handler: None,
        }
    }

    /// Sets the key used to replace and cancel this task within its resource scope.
    ///
    /// Spawning another task with the same scoped key cancels the previous task.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Sets a user-facing label included in task status snapshots.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets a runtime-clock timeout for each attempt, starting when it is submitted.
    ///
    /// Timeout cancels result delivery and signals the token; it does not stop a
    /// blocking worker that ignores cancellation.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the retry budget and delay used when the runner returns [`TaskOutcome::Retry`].
    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Maps every task-status notification to an application message.
    pub fn on_status<F>(mut self, handler: F) -> Self
    where
        F: Fn(TaskStatus) -> M + Send + Sync + 'static,
    {
        self.status_handler = Some(Arc::new(move |status| Some(handler(status))));
        self
    }

    /// Maps task-status notifications to optional messages; `None` suppresses a notification.
    pub fn on_status_opt<F>(mut self, handler: F) -> Self
    where
        F: Fn(TaskStatus) -> Option<M> + Send + Sync + 'static,
    {
        self.status_handler = Some(Arc::new(handler));
        self
    }
    /// Maps emitted messages and all task/request status callbacks.
    pub fn map<N, F>(self, map: F) -> Task<N>
    where
        M: Send + 'static,
        N: Send + 'static,
        F: Fn(M) -> N + Send + Sync + 'static,
    {
        let map = Arc::new(map);
        let runner = self.runner;
        let runner_map = map.clone();
        Task {
            key: self.key,
            label: self.label,
            timeout: self.timeout,
            retry: self.retry,
            status_handler: self.status_handler.map(|handler| {
                Arc::new(move |status| handler(status).map(|m| map(m))) as StatusHandler<N>
            }),
            runner: Arc::new(move |ctx: TaskContext<N>| {
                let id = ctx.task_id;
                let attempt = ctx.attempt;
                let cancellation = ctx.cancellation.clone();
                let map = runner_map.clone();
                runner(TaskContext {
                    task_id: id,
                    attempt,
                    cancellation,
                    sender: Arc::new(move |m| ctx.emit(map(m))),
                })
            }),
        }
    }
}

impl<M> std::fmt::Debug for Task<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("key", &self.key)
            .field("label", &self.label)
            .field("timeout", &self.timeout)
            .field("retry", &self.retry)
            .finish()
    }
}

pub(crate) struct TaskEventSender<M> {
    sender: Sender<TaskEvent<M>>,
    wake: crate::render::WakeHandle,
    origin: Option<(TaskId, u32)>,
}

impl<M> Clone for TaskEventSender<M> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            wake: self.wake.clone(),
            origin: self.origin,
        }
    }
}

impl<M> TaskEventSender<M> {
    pub(crate) fn new(sender: Sender<TaskEvent<M>>, wake: crate::render::WakeHandle) -> Self {
        Self {
            sender,
            wake,
            origin: None,
        }
    }

    pub(crate) fn for_attempt(&self, id: TaskId, attempt: u32) -> Self {
        let mut sender = self.clone();
        sender.origin = Some((id, attempt));
        sender
    }

    pub(crate) fn send_message(&self, message: M) -> Result<(), std::sync::mpsc::SendError<M>> {
        self.sender
            .send(TaskEvent::Message(message))
            .map_err(|error| {
                let TaskEvent::Message(message) = error.0 else {
                    unreachable!()
                };
                std::sync::mpsc::SendError(message)
            })?;
        self.wake.wake();
        Ok(())
    }

    pub(crate) fn send(&self, event: TaskEvent<M>) {
        let event = match (self.origin, event) {
            (Some((id, attempt)), TaskEvent::Message(message)) => TaskEvent::TaskMessage {
                id,
                attempt,
                message,
            },
            (_, event) => event,
        };
        if self.sender.send(event).is_ok() {
            self.wake.wake();
        }
    }
}

#[derive(Debug)]
pub(crate) enum TaskEvent<M> {
    Message(M),
    TaskMessage {
        id: TaskId,
        attempt: u32,
        message: M,
    },
    Finished {
        id: TaskId,
        attempt: u32,
        state: TaskState,
    },
}

/// Context for one task attempt, including message emission and cancellation.
pub struct TaskContext<M> {
    task_id: TaskId,
    attempt: u32,
    cancellation: CancellationToken,
    sender: Arc<dyn Fn(M) + Send + Sync>,
}

impl<M> TaskContext<M> {
    pub(crate) fn new(
        task_id: TaskId,
        attempt: u32,
        cancellation: CancellationToken,
        sender: TaskEventSender<M>,
    ) -> Self
    where
        M: Send + 'static,
    {
        Self {
            task_id,
            attempt,
            cancellation,
            sender: Arc::new(move |message| sender.send(TaskEvent::Message(message))),
        }
    }

    /// Returns the task identity shared by all retry attempts.
    pub fn task_id(&self) -> TaskId {
        self.task_id
    }

    /// Returns the one-based attempt number.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Sends an application message through the task's runtime channel.
    ///
    /// Delivery is asynchronous; the runtime rejects results from cancelled or
    /// superseded attempts.
    pub fn emit(&self, message: M) {
        (self.sender)(message);
    }

    /// Returns a clone of the attempt's shared cancellation token.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    /// Returns whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    /// Blocks for up to `duration`, checking cancellation at intervals of at most 10 ms.
    ///
    /// Returns `true` if the wait finishes without cancellation, or `false` when
    /// cancelled. This uses wall-clock sleep, independent of an injected runtime clock.
    pub fn sleep(&self, duration: Duration) -> bool {
        sleep_with_cancellation(&self.cancellation, duration)
    }
}

/// Typed result returned by one request attempt.
#[derive(Debug, Clone)]
pub enum RequestOutcome<T, E> {
    /// Completes the request with a value.
    Success(T),
    /// Reports a failed attempt; the retry policy determines whether to try again.
    Failure(E),
    /// Completes the request as cancelled.
    Cancelled,
}

impl<T, E> RequestOutcome<T, E> {
    /// Creates a successful outcome carrying `value`.
    pub fn success(value: T) -> Self {
        Self::Success(value)
    }

    /// Creates a failed attempt carrying `error`.
    pub fn failure(error: E) -> Self {
        Self::Failure(error)
    }

    /// Creates a cancelled outcome.
    pub fn cancelled() -> Self {
        Self::Cancelled
    }
}

/// Lifecycle notification for a typed request, mapped into an application message.
#[derive(Debug, Clone)]
pub enum RequestEvent<T, E> {
    /// The request task was registered.
    Queued {
        /// Identity of this request, shared by all its lifecycle events.
        request_id: RequestId,
        /// Task-status snapshot associated with the event.
        status: TaskStatus,
    },
    /// An attempt was submitted to the executor.
    Started {
        /// Identity of this request, shared by all its lifecycle events.
        request_id: RequestId,
        /// Task-status snapshot associated with the event.
        status: TaskStatus,
    },
    /// The request is waiting to retry after a failed attempt.
    Retrying {
        /// Identity of this request, shared by all its lifecycle events.
        request_id: RequestId,
        /// Task-status snapshot associated with the event.
        status: TaskStatus,
    },
    /// The request produced its value.
    Succeeded {
        /// Identity of this request, shared by all its lifecycle events.
        request_id: RequestId,
        /// Runtime task identity that produced the result.
        task_id: TaskId,
        /// Successful request result.
        value: T,
    },
    /// The request exhausted its retry budget, carrying the final error.
    Failed {
        /// Identity of this request, shared by all its lifecycle events.
        request_id: RequestId,
        /// Task-status snapshot associated with the event.
        status: TaskStatus,
        /// Error from the final failed attempt.
        error: E,
    },
    /// The request was cancelled.
    Cancelled {
        /// Identity of this request, shared by all its lifecycle events.
        request_id: RequestId,
        /// Task-status snapshot associated with the event.
        status: TaskStatus,
    },
    /// The current attempt exceeded its timeout.
    TimedOut {
        /// Identity of this request, shared by all its lifecycle events.
        request_id: RequestId,
        /// Task-status snapshot associated with the event.
        status: TaskStatus,
    },
    /// The request runner panicked.
    Panicked {
        /// Identity of this request, shared by all its lifecycle events.
        request_id: RequestId,
        /// Task-status snapshot associated with the event.
        status: TaskStatus,
    },
}

impl<T, E> RequestEvent<T, E> {
    /// Returns the request identity carried by this event.
    pub fn request_id(&self) -> RequestId {
        match self {
            RequestEvent::Queued { request_id, .. }
            | RequestEvent::Started { request_id, .. }
            | RequestEvent::Retrying { request_id, .. }
            | RequestEvent::Succeeded { request_id, .. }
            | RequestEvent::Failed { request_id, .. }
            | RequestEvent::Cancelled { request_id, .. }
            | RequestEvent::TimedOut { request_id, .. }
            | RequestEvent::Panicked { request_id, .. } => *request_id,
        }
    }

    /// Returns the underlying task identity.
    pub fn task_id(&self) -> TaskId {
        match self {
            RequestEvent::Queued { status, .. }
            | RequestEvent::Started { status, .. }
            | RequestEvent::Retrying { status, .. }
            | RequestEvent::Failed { status, .. }
            | RequestEvent::Cancelled { status, .. }
            | RequestEvent::TimedOut { status, .. }
            | RequestEvent::Panicked { status, .. } => status.id,
            RequestEvent::Succeeded { task_id, .. } => *task_id,
        }
    }

    /// Returns the status snapshot, or `None` for a successful result.
    pub fn status(&self) -> Option<&TaskStatus> {
        match self {
            RequestEvent::Queued { status, .. }
            | RequestEvent::Started { status, .. }
            | RequestEvent::Retrying { status, .. }
            | RequestEvent::Failed { status, .. }
            | RequestEvent::Cancelled { status, .. }
            | RequestEvent::TimedOut { status, .. }
            | RequestEvent::Panicked { status, .. } => Some(status),
            RequestEvent::Succeeded { .. } => None,
        }
    }
}

/// Application-facing state of a typed request.
#[derive(Debug, Clone, Default)]
pub enum RequestPhase<T, E> {
    /// No request event has been applied (default).
    #[default]
    Idle,
    /// The request is queued, running, or waiting to retry.
    Loading,
    /// The request completed with a value.
    Success(T),
    /// The request failed with an error after exhausting retries.
    Failed(E),
    /// The request was cancelled.
    Cancelled,
    /// The request timed out.
    TimedOut,
    /// The request runner panicked.
    Panicked,
}

impl<T, E> RequestPhase<T, E> {
    /// Returns whether a request is queued, running, or waiting to retry.
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Returns whether the phase is success, failure, cancellation, timeout, or panic.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Success(_) | Self::Failed(_) | Self::Cancelled | Self::TimedOut | Self::Panicked
        )
    }
}

/// Latest request lifecycle and result, with stale-request filtering.
///
/// Call [`Self::apply`] for each request event. Events with an older request ID
/// are ignored; events for the same or a newer ID are accepted. Successful
/// results replace the task status snapshot.
///
/// # Examples
///
/// ```
/// use rsille::tui::{
///     app::App,
///     effect::{Request, RequestEvent, RequestOutcome, RequestState},
///     runtime::ManualExecutor,
///     widgets::label,
/// };
///
/// enum Message { Load, Loaded(RequestEvent<u32, String>) }
/// let executor = ManualExecutor::default();
/// let mut runtime = App::new(RequestState::<u32, String>::new())
///     .with_executor(executor.clone())
///     .update(|state, message, ctx| match message {
///         Message::Load => ctx.request(
///             Request::new(|_| RequestOutcome::Success(42)).key("answer"),
///             Message::Loaded,
///         ),
///         Message::Loaded(event) => { state.apply(event); }
///     })
///     .view(|state| label(format!("{:?}", state.value())))
///     .build()
///     .size(12, 1);
///
/// runtime.handle().send(Message::Load).unwrap();
/// runtime.step(&[]).unwrap();
/// assert!(executor.run_next());
/// runtime.step(&[]).unwrap();
/// assert_eq!(runtime.state().value(), Some(&42));
/// ```
#[derive(Debug, Clone)]
pub struct RequestState<T, E> {
    request_id: Option<RequestId>,
    task_id: Option<TaskId>,
    status: Option<TaskStatus>,
    phase: RequestPhase<T, E>,
}

impl<T, E> Default for RequestState<T, E> {
    fn default() -> Self {
        Self {
            request_id: None,
            task_id: None,
            status: None,
            phase: RequestPhase::Idle,
        }
    }
}

impl<T, E> RequestState<T, E> {
    /// Creates idle state with no request, task, status, or result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the identity of the most recently accepted request.
    pub fn request_id(&self) -> Option<RequestId> {
        self.request_id
    }

    /// Returns the task identity from the most recently accepted event.
    pub fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    /// Returns the last accepted status snapshot; successful results clear it.
    pub fn status(&self) -> Option<&TaskStatus> {
        self.status.as_ref()
    }

    /// Returns the current application-facing request phase.
    pub fn phase(&self) -> &RequestPhase<T, E> {
        &self.phase
    }

    /// Returns whether the request is queued, running, or waiting to retry.
    pub fn is_loading(&self) -> bool {
        self.phase.is_loading()
    }

    /// Returns the result only while the phase is success.
    pub fn value(&self) -> Option<&T> {
        match &self.phase {
            RequestPhase::Success(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the error only while the phase is failed.
    pub fn error(&self) -> Option<&E> {
        match &self.phase {
            RequestPhase::Failed(error) => Some(error),
            _ => None,
        }
    }

    /// Resets to idle and forgets IDs and results without cancelling any running task.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Applies an event unless its request ID is older than the last accepted ID.
    ///
    /// Returns `true` for an accepted event, even if its phase is unchanged, and
    /// `false` for a stale event. This compares request IDs, not attempt numbers.
    pub fn apply(&mut self, event: RequestEvent<T, E>) -> bool {
        let request_id = event.request_id();
        if self.request_id.is_some_and(|current| request_id < current) {
            return false;
        }

        self.request_id = Some(request_id);
        self.task_id = Some(event.task_id());

        match event {
            RequestEvent::Queued { status, .. }
            | RequestEvent::Started { status, .. }
            | RequestEvent::Retrying { status, .. } => {
                self.status = Some(status);
                self.phase = RequestPhase::Loading;
            }
            RequestEvent::Succeeded { value, .. } => {
                self.status = None;
                self.phase = RequestPhase::Success(value);
            }
            RequestEvent::Failed { status, error, .. } => {
                self.status = Some(status);
                self.phase = RequestPhase::Failed(error);
            }
            RequestEvent::Cancelled { status, .. } => {
                self.status = Some(status);
                self.phase = RequestPhase::Cancelled;
            }
            RequestEvent::TimedOut { status, .. } => {
                self.status = Some(status);
                self.phase = RequestPhase::TimedOut;
            }
            RequestEvent::Panicked { status, .. } => {
                self.status = Some(status);
                self.phase = RequestPhase::Panicked;
            }
        }

        true
    }
}

/// Task identity, attempt number, and cooperative cancellation for a request runner.
#[derive(Debug, Clone)]
pub struct RequestContext {
    task_id: TaskId,
    attempt: u32,
    cancellation: CancellationToken,
}

impl RequestContext {
    fn new(task_id: TaskId, attempt: u32, cancellation: CancellationToken) -> Self {
        Self {
            task_id,
            attempt,
            cancellation,
        }
    }

    /// Returns the task identity shared by all retry attempts.
    pub fn task_id(&self) -> TaskId {
        self.task_id
    }

    /// Returns the one-based attempt number.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Returns a clone of the attempt's shared cancellation token.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    /// Returns whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    /// Blocks for up to `duration`, checking cancellation at intervals of at most 10 ms.
    ///
    /// Returns `true` if the wait finishes without cancellation, or `false` when
    /// cancelled. This uses wall-clock sleep, independent of an injected runtime clock.
    pub fn sleep(&self, duration: Duration) -> bool {
        sleep_with_cancellation(&self.cancellation, duration)
    }
}

type RequestRunner<T, E> =
    Arc<dyn Fn(RequestContext) -> RequestOutcome<T, E> + Send + Sync + 'static>;

/// A repeatable blocking request returning a typed value or error.
///
/// Convert it with [`Self::into_task`] or schedule it with [`UpdateCtx::request`].
/// Failures request another attempt according to the retry policy; the final
/// error is emitted only after retries are exhausted.
pub struct Request<T, E> {
    key: Option<String>,
    label: Option<String>,
    timeout: Option<Duration>,
    retry: RetryPolicy,
    runner: RequestRunner<T, E>,
}

impl<T, E> Request<T, E> {
    /// Creates an unkeyed request with no timeout or retries.
    pub fn new<F>(runner: F) -> Self
    where
        F: Fn(RequestContext) -> RequestOutcome<T, E> + Send + Sync + 'static,
    {
        Self {
            key: None,
            label: None,
            timeout: None,
            retry: RetryPolicy::none(),
            runner: Arc::new(runner),
        }
    }

    /// Sets the scoped task key; scheduling a replacement cancels the previous task.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Sets the user-facing label in task status snapshots.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the runtime-clock timeout for each submitted attempt.
    ///
    /// The runner must cooperate with cancellation to stop blocking work.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the retry policy applied to failed request outcomes.
    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Converts the request into a task, allocating a fresh request ID.
    ///
    /// The mapper converts lifecycle events and the final result into application
    /// messages. The request's key, label, timeout, and retry policy are preserved.
    pub fn into_task<M, F>(self, map: F) -> Task<M>
    where
        T: Send + 'static,
        E: Clone + Send + 'static,
        M: Send + 'static,
        F: Fn(RequestEvent<T, E>) -> M + Send + Sync + 'static,
    {
        let request_id = RequestId::next();
        let map = Arc::new(map);
        let runner = self.runner.clone();
        let failure = Arc::new(Mutex::new(None::<E>));

        let runner_map = map.clone();
        let runner_failure = failure.clone();
        let mut task = Task::new(move |task| {
            let mut slot = runner_failure
                .lock()
                .expect("request failure slot should not be poisoned");
            *slot = None;
            drop(slot);

            let request_ctx =
                RequestContext::new(task.task_id(), task.attempt(), task.cancellation_token());

            match runner(request_ctx) {
                RequestOutcome::Success(value) => {
                    task.emit(runner_map(RequestEvent::Succeeded {
                        request_id,
                        task_id: task.task_id(),
                        value,
                    }));
                    TaskOutcome::Complete
                }
                RequestOutcome::Failure(error) => {
                    *runner_failure
                        .lock()
                        .expect("request failure slot should not be poisoned") = Some(error);
                    TaskOutcome::Retry
                }
                RequestOutcome::Cancelled => TaskOutcome::Cancelled,
            }
        })
        .retry(self.retry)
        .on_status_opt({
            let status_map = map.clone();
            let status_failure = failure.clone();
            move |status| match status.state {
                TaskState::Queued => Some(status_map(RequestEvent::Queued { request_id, status })),
                TaskState::Running => {
                    Some(status_map(RequestEvent::Started { request_id, status }))
                }
                TaskState::RetryScheduled => {
                    Some(status_map(RequestEvent::Retrying { request_id, status }))
                }
                TaskState::Cancelled => {
                    Some(status_map(RequestEvent::Cancelled { request_id, status }))
                }
                TaskState::TimedOut => {
                    Some(status_map(RequestEvent::TimedOut { request_id, status }))
                }
                TaskState::Panicked => {
                    Some(status_map(RequestEvent::Panicked { request_id, status }))
                }
                TaskState::Failed => {
                    let error = status_failure
                        .lock()
                        .expect("request failure slot should not be poisoned")
                        .clone()
                        .expect("request failures should store an error payload");
                    Some(status_map(RequestEvent::Failed {
                        request_id,
                        status,
                        error,
                    }))
                }
                TaskState::Completed | TaskState::Cancelling => None,
            }
        });

        if let Some(key) = self.key {
            task = task.key(key);
        }

        if let Some(label) = self.label {
            task = task.label(label);
        }

        if let Some(timeout) = self.timeout {
            task = task.timeout(timeout);
        }

        task
    }
}

/// Effect queue and task-status snapshot available during application updates.
///
/// Methods enqueue work for the runtime; they do not execute blocking jobs
/// inline. Task queries describe the snapshot taken for this update, so newly
/// queued effects are not reflected immediately.
pub struct UpdateCtx<'a, M> {
    effects: &'a mut Vec<Effect<M>>,
    task_statuses: Vec<TaskStatus>,
    task_keys: HashMap<ResourceKey, TaskId>,
    now: Instant,
}

impl<'a, M> UpdateCtx<'a, M> {
    pub(crate) fn new(
        effects: &'a mut Vec<Effect<M>>,
        task_statuses: Vec<TaskStatus>,
        task_keys: HashMap<ResourceKey, TaskId>,
        now: Instant,
    ) -> Self {
        Self {
            effects,
            task_statuses,
            task_keys,
            now,
        }
    }

    /// Returns the runtime-clock instant captured for this update.
    pub fn now(&self) -> Instant {
        self.now
    }

    /// Finish the application after this update, cancelling resources.
    pub fn exit(&mut self) {
        self.effects.push(Effect::Exit);
    }

    /// Queues a message for a subsequent application update.
    pub fn emit(&mut self, message: M) {
        self.effects.push(Effect::message(message));
    }

    /// Renders this update's final view and leaves it above the next inline frame.
    ///
    /// Message processing and later effects pause until presentation and cursor
    /// advancement succeed. `after_message` runs before other queued messages;
    /// use it to reset the result state to the next prompt. This does not snapshot
    /// state at the call site: the entire current update finishes first.
    ///
    /// Terminal execution requires inline mode (fullscreen returns an error).
    /// The cursor advances with real newlines, including native raster padding.
    /// Sixel retention while scrolling depends on the terminal; this API does not
    /// guarantee graphics in scrollback. Headless execution reports a committed
    /// frame through [`crate::tui::runtime::StepResult::committed`], retaining it
    /// in `Runtime::frame()` until the next step. No terminal I/O occurs there.
    ///
    /// ```
    /// use rsille::tui::{app::App, widgets::label};
    ///
    /// let mut app = App::new("prompt>".to_owned()).build_with_effects(
    ///     |state, submit: bool, ctx| {
    ///         if submit {
    ///             *state = "result: 42".into();
    ///             ctx.commit_frame(false);
    ///         } else {
    ///             *state = "prompt>".into();
    ///         }
    ///     },
    ///     |state| label(state.clone()),
    /// ).size(20, 1);
    /// app.handle().send(true).unwrap();
    /// assert!(app.step(&[]).unwrap().committed);
    /// assert_eq!(app.state(), "result: 42"); // frame() contains the result
    /// assert!(!app.step(&[]).unwrap().committed);
    /// assert_eq!(app.state(), "prompt>");
    /// // For terminal execution, use app.run_inline() instead of stepping.
    /// ```
    pub fn commit_frame(&mut self, after_message: M) {
        self.effects.push(Effect::commit_frame(after_message));
    }

    /// Appends an effect to the update's effect queue.
    pub fn dispatch(&mut self, effect: Effect<M>) {
        self.effects.push(effect);
    }

    /// Queues a task for execution after the update.
    pub fn spawn(&mut self, task: Task<M>) {
        self.effects.push(Effect::spawn(task));
    }

    /// Queues a message for delivery after a runtime-clock delay.
    pub fn after(&mut self, duration: Duration, message: M) {
        self.effects.push(Effect::after(duration, message));
    }

    /// Queues a delayed message, replacing the timer and payload for the same root-scope key.
    pub fn debounce(&mut self, key: impl Into<String>, duration: Duration, message: M) {
        self.effects.push(Effect::debounce(key, duration, message));
    }

    /// Queues cancellation of a task by identity.
    pub fn cancel_task(&mut self, task_id: TaskId) {
        self.effects.push(Effect::cancel_task(task_id));
    }

    /// Queues cancellation of a task by its root-scope key.
    pub fn cancel_task_key(&mut self, key: impl Into<String>) {
        self.effects.push(Effect::cancel_task_key(key));
    }

    /// Queues cancellation of a debounce by its root-scope key.
    pub fn cancel_debounce(&mut self, key: impl Into<String>) {
        self.effects.push(Effect::cancel_debounce(key));
    }

    /// Returns task-status snapshots captured for this update, including retained terminal tasks.
    pub fn tasks(&self) -> &[TaskStatus] {
        &self.task_statuses
    }

    /// Looks up a task identity in the current update's status snapshot.
    pub fn task_status(&self, task_id: TaskId) -> Option<&TaskStatus> {
        self.task_statuses
            .iter()
            .find(|status| status.id == task_id)
    }

    /// Looks up the latest task for a key in the root resource scope.
    pub fn task_status_by_key(&self, key: &str) -> Option<&TaskStatus> {
        let task_id = self.task_keys.get(&ResourceKey::new(Scope::root(), key))?;
        self.task_status(*task_id)
    }

    /// Looks up the latest task for a key in the specified resource scope.
    pub fn task_status_in_scope(&self, scope: &Scope, key: &str) -> Option<&TaskStatus> {
        let id = self.task_keys.get(&ResourceKey::new(scope.clone(), key))?;
        self.task_status(*id)
    }

    /// Returns whether the root-scope keyed task is active in this update's snapshot.
    pub fn is_task_active(&self, key: &str) -> bool {
        self.task_status_by_key(key)
            .is_some_and(TaskStatus::is_active)
    }

    /// Returns whether the root-scope keyed task is running in this update's snapshot.
    pub fn is_task_running(&self, key: &str) -> bool {
        self.task_status_by_key(key)
            .is_some_and(TaskStatus::is_running)
    }

    /// Returns whether the root-scope keyed task is queued or waiting to retry.
    pub fn is_task_pending(&self, key: &str) -> bool {
        self.task_status_by_key(key)
            .is_some_and(TaskStatus::is_pending)
    }

    /// Converts a request into a task and queues it, mapping lifecycle events to messages.
    pub fn request<T, E, F>(&mut self, request: Request<T, E>, map: F)
    where
        T: Send + 'static,
        E: Clone + Send + 'static,
        M: Send + 'static,
        F: Fn(RequestEvent<T, E>) -> M + Send + Sync + 'static,
    {
        self.spawn(request.into_task(map));
    }
}

pub(crate) fn run_task_attempt<M: Send + 'static>(
    context: TaskContext<M>,
    runner: TaskRunner<M>,
) -> TaskState {
    if context.is_cancelled() {
        return TaskState::Cancelled;
    }
    let cancellation = context.cancellation_token();
    let outcome = catch_unwind(AssertUnwindSafe(|| runner(context)));
    if cancellation.is_cancelled() {
        return TaskState::Cancelled;
    }
    match outcome {
        Ok(TaskOutcome::Complete) => TaskState::Completed,
        Ok(TaskOutcome::Cancelled) => TaskState::Cancelled,
        Ok(TaskOutcome::Retry) => TaskState::RetryScheduled,
        Err(_) => TaskState::Panicked,
    }
}

pub(crate) fn sleep_with_cancellation(
    cancellation: &CancellationToken,
    duration: Duration,
) -> bool {
    if duration.is_zero() {
        return !cancellation.is_cancelled();
    }

    let started_at = Instant::now();
    while started_at.elapsed() < duration {
        if cancellation.is_cancelled() {
            return false;
        }

        let remaining = duration.saturating_sub(started_at.elapsed());
        thread::sleep(remaining.min(Duration::from_millis(10)));
    }

    !cancellation.is_cancelled()
}

#[cfg(test)]
mod tests {
    use super::{
        CancellationToken, RequestEvent, RequestId, RequestPhase, RequestState, RetryPolicy,
        TaskId, TaskState, TaskStatus,
    };
    use std::time::Duration;
    use std::time::Instant;

    #[test]
    fn retry_policy_reports_attempt_budget() {
        let policy = RetryPolicy::fixed(2, Duration::from_millis(10));
        assert_eq!(policy.max_attempts(), 3);
        assert!(policy.can_retry(1));
        assert!(policy.can_retry(2));
        assert!(!policy.can_retry(3));
    }

    #[test]
    fn task_state_marks_terminal_variants() {
        assert!(TaskState::Completed.is_terminal());
        assert!(TaskState::Cancelled.is_terminal());
        assert!(!TaskState::Running.is_terminal());
    }

    #[test]
    fn cancellation_token_tracks_requests() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn request_state_ignores_stale_events() {
        let created_at = Instant::now();
        let mut state = RequestState::<u32, &'static str>::new();

        assert!(state.apply(RequestEvent::Started {
            request_id: RequestId(2),
            status: TaskStatus::new(
                TaskId(2),
                Some("search".into()),
                None,
                TaskState::Running,
                1,
                1,
                created_at,
            ),
        }));
        assert!(state.is_loading());

        assert!(!state.apply(RequestEvent::Succeeded {
            request_id: RequestId(1),
            task_id: TaskId(1),
            value: 7,
        }));
        assert!(state.is_loading());

        assert!(state.apply(RequestEvent::Succeeded {
            request_id: RequestId(2),
            task_id: TaskId(2),
            value: 42,
        }));
        assert_eq!(state.value(), Some(&42));
    }

    #[test]
    fn task_status_helpers_match_lifecycle() {
        let created_at = Instant::now();
        let running = TaskStatus::new(TaskId(1), None, None, TaskState::Running, 1, 1, created_at);
        let queued = TaskStatus::new(TaskId(2), None, None, TaskState::Queued, 1, 1, created_at);
        let failed = TaskStatus::new(TaskId(3), None, None, TaskState::Failed, 1, 1, created_at);

        assert!(running.is_active());
        assert!(running.is_running());
        assert!(!running.is_pending());
        assert!(queued.is_pending());
        assert!(!failed.is_active());
    }

    #[test]
    fn request_phase_flags_loading_and_terminal_states() {
        assert!(RequestPhase::<u32, &'static str>::Loading.is_loading());
        assert!(RequestPhase::<u32, &'static str>::Success(1).is_terminal());
        assert!(RequestPhase::<u32, &'static str>::Failed("boom").is_terminal());
        assert!(!RequestPhase::<u32, &'static str>::Idle.is_terminal());
    }
}
