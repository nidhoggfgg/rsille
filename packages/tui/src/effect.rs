use crate::scope::{ResourceKey, Scope};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc::Sender, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub(crate) u64);

impl TaskId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestId(u64);

impl RequestId {
    fn next() -> Self {
        static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Queued,
    Running,
    RetryScheduled,
    Cancelling,
    Completed,
    Cancelled,
    TimedOut,
    Failed,
    Panicked,
}

impl TaskState {
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

#[derive(Debug, Clone)]
pub struct TaskStatus {
    pub id: TaskId,
    pub key: Option<String>,
    pub label: Option<String>,
    /// Namespace of the owning component instance.
    pub scope: Scope,
    pub state: TaskState,
    pub attempt: u32,
    pub max_attempts: u32,
    pub created_at: Instant,
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

    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            TaskState::Queued
                | TaskState::Running
                | TaskState::RetryScheduled
                | TaskState::Cancelling
        )
    }

    pub fn is_running(&self) -> bool {
        self.state == TaskState::Running
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.state, TaskState::Queued | TaskState::RetryScheduled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskOutcome {
    #[default]
    Complete,
    Retry,
    Cancelled,
}

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
    pub const fn none() -> Self {
        Self {
            max_retries: 0,
            delay: Duration::ZERO,
        }
    }

    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            delay: Duration::ZERO,
        }
    }

    pub fn fixed(max_retries: u32, delay: Duration) -> Self {
        Self { max_retries, delay }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn max_retries(self) -> u32 {
        self.max_retries
    }

    pub fn delay(self) -> Duration {
        self.delay
    }

    pub fn max_attempts(self) -> u32 {
        self.max_retries.saturating_add(1)
    }

    pub(crate) fn can_retry(self, attempt: u32) -> bool {
        attempt <= self.max_retries
    }
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub enum Effect<M> {
    None,
    Message(M),
    Batch(Vec<Effect<M>>),
    Spawn(Task<M>),
    CancelTask(TaskId),
    CancelTaskKey(String),
    After(Duration, M),
    Debounce {
        key: String,
        duration: Duration,
        message: M,
    },
    CancelDebounce(String),
    Scoped(Scope, Box<Effect<M>>),
}

impl<M> Effect<M> {
    pub fn none() -> Self {
        Self::None
    }

    pub fn message(message: M) -> Self {
        Self::Message(message)
    }

    pub fn batch(effects: impl IntoIterator<Item = Effect<M>>) -> Self {
        Self::Batch(effects.into_iter().collect())
    }

    pub fn spawn(task: Task<M>) -> Self {
        Self::Spawn(task)
    }

    pub fn cancel_task(task_id: TaskId) -> Self {
        Self::CancelTask(task_id)
    }

    pub fn cancel_task_key(key: impl Into<String>) -> Self {
        Self::CancelTaskKey(key.into())
    }

    pub fn after(duration: Duration, message: M) -> Self {
        Self::After(duration, message)
    }

    pub fn debounce(key: impl Into<String>, duration: Duration, message: M) -> Self {
        Self::Debounce {
            key: key.into(),
            duration,
            message,
        }
    }

    pub fn cancel_debounce(key: impl Into<String>) -> Self {
        Self::CancelDebounce(key.into())
    }
    /// Attach resources and messages to a mounted widget scope.
    pub fn scope(self, scope: impl Into<Scope>) -> Self {
        Self::Scoped(scope.into(), Box::new(self))
    }

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
            Self::Message(m) => Effect::Message(map(m)),
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

pub struct Task<M> {
    pub(crate) key: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) retry: RetryPolicy,
    pub(crate) runner: TaskRunner<M>,
    pub(crate) status_handler: Option<StatusHandler<M>>,
}

impl<M> Task<M> {
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

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub fn on_status<F>(mut self, handler: F) -> Self
    where
        F: Fn(TaskStatus) -> M + Send + Sync + 'static,
    {
        self.status_handler = Some(Arc::new(move |status| Some(handler(status))));
        self
    }

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
    wake: render::WakeHandle,
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
    pub(crate) fn new(sender: Sender<TaskEvent<M>>, wake: render::WakeHandle) -> Self {
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

    pub fn task_id(&self) -> TaskId {
        self.task_id
    }

    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    pub fn emit(&self, message: M) {
        (self.sender)(message);
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    pub fn sleep(&self, duration: Duration) -> bool {
        sleep_with_cancellation(&self.cancellation, duration)
    }
}

#[derive(Debug, Clone)]
pub enum RequestOutcome<T, E> {
    Success(T),
    Failure(E),
    Cancelled,
}

impl<T, E> RequestOutcome<T, E> {
    pub fn success(value: T) -> Self {
        Self::Success(value)
    }

    pub fn failure(error: E) -> Self {
        Self::Failure(error)
    }

    pub fn cancelled() -> Self {
        Self::Cancelled
    }
}

#[derive(Debug, Clone)]
pub enum RequestEvent<T, E> {
    Queued {
        request_id: RequestId,
        status: TaskStatus,
    },
    Started {
        request_id: RequestId,
        status: TaskStatus,
    },
    Retrying {
        request_id: RequestId,
        status: TaskStatus,
    },
    Succeeded {
        request_id: RequestId,
        task_id: TaskId,
        value: T,
    },
    Failed {
        request_id: RequestId,
        status: TaskStatus,
        error: E,
    },
    Cancelled {
        request_id: RequestId,
        status: TaskStatus,
    },
    TimedOut {
        request_id: RequestId,
        status: TaskStatus,
    },
    Panicked {
        request_id: RequestId,
        status: TaskStatus,
    },
}

impl<T, E> RequestEvent<T, E> {
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

#[derive(Debug, Clone, Default)]
pub enum RequestPhase<T, E> {
    #[default]
    Idle,
    Loading,
    Success(T),
    Failed(E),
    Cancelled,
    TimedOut,
    Panicked,
}

impl<T, E> RequestPhase<T, E> {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Success(_) | Self::Failed(_) | Self::Cancelled | Self::TimedOut | Self::Panicked
        )
    }
}

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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_id(&self) -> Option<RequestId> {
        self.request_id
    }

    pub fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    pub fn status(&self) -> Option<&TaskStatus> {
        self.status.as_ref()
    }

    pub fn phase(&self) -> &RequestPhase<T, E> {
        &self.phase
    }

    pub fn is_loading(&self) -> bool {
        self.phase.is_loading()
    }

    pub fn value(&self) -> Option<&T> {
        match &self.phase {
            RequestPhase::Success(value) => Some(value),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&E> {
        match &self.phase {
            RequestPhase::Failed(error) => Some(error),
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

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

    pub fn task_id(&self) -> TaskId {
        self.task_id
    }

    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    pub fn sleep(&self, duration: Duration) -> bool {
        sleep_with_cancellation(&self.cancellation, duration)
    }
}

type RequestRunner<T, E> =
    Arc<dyn Fn(RequestContext) -> RequestOutcome<T, E> + Send + Sync + 'static>;

pub struct Request<T, E> {
    key: Option<String>,
    label: Option<String>,
    timeout: Option<Duration>,
    retry: RetryPolicy,
    runner: RequestRunner<T, E>,
}

impl<T, E> Request<T, E> {
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

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

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

    pub fn now(&self) -> Instant {
        self.now
    }

    pub fn emit(&mut self, message: M) {
        self.effects.push(Effect::message(message));
    }

    pub fn dispatch(&mut self, effect: Effect<M>) {
        self.effects.push(effect);
    }

    pub fn spawn(&mut self, task: Task<M>) {
        self.effects.push(Effect::spawn(task));
    }

    pub fn after(&mut self, duration: Duration, message: M) {
        self.effects.push(Effect::after(duration, message));
    }

    pub fn debounce(&mut self, key: impl Into<String>, duration: Duration, message: M) {
        self.effects.push(Effect::debounce(key, duration, message));
    }

    pub fn cancel_task(&mut self, task_id: TaskId) {
        self.effects.push(Effect::cancel_task(task_id));
    }

    pub fn cancel_task_key(&mut self, key: impl Into<String>) {
        self.effects.push(Effect::cancel_task_key(key));
    }

    pub fn cancel_debounce(&mut self, key: impl Into<String>) {
        self.effects.push(Effect::cancel_debounce(key));
    }

    pub fn tasks(&self) -> &[TaskStatus] {
        &self.task_statuses
    }

    pub fn task_status(&self, task_id: TaskId) -> Option<&TaskStatus> {
        self.task_statuses
            .iter()
            .find(|status| status.id == task_id)
    }

    pub fn task_status_by_key(&self, key: &str) -> Option<&TaskStatus> {
        let task_id = self.task_keys.get(&ResourceKey::new(Scope::root(), key))?;
        self.task_status(*task_id)
    }

    pub fn task_status_in_scope(&self, scope: &Scope, key: &str) -> Option<&TaskStatus> {
        let id = self.task_keys.get(&ResourceKey::new(scope.clone(), key))?;
        self.task_status(*id)
    }

    pub fn is_task_active(&self, key: &str) -> bool {
        self.task_status_by_key(key)
            .is_some_and(TaskStatus::is_active)
    }

    pub fn is_task_running(&self, key: &str) -> bool {
        self.task_status_by_key(key)
            .is_some_and(TaskStatus::is_running)
    }

    pub fn is_task_pending(&self, key: &str) -> bool {
        self.task_status_by_key(key)
            .is_some_and(TaskStatus::is_pending)
    }

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
