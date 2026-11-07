//! Terminal-independent application execution, clocks and task executors.
//!
//! Build a [`Runtime`] with [`App::build`](crate::tui::app::App::build), then
//! call [`Runtime::step`] to dispatch input and render into an in-memory buffer.
//! Building and stepping do not configure a terminal. An embedding host drives
//! each step and reads the output through [`Runtime::frame`]. Alternatively,
//! [`Runtime::run`] transfers control to the terminal event loop.
//!
//! Inject a [`ManualClock`] and [`ManualExecutor`] through
//! [`App::with_clock`](crate::tui::app::App::with_clock) and
//! [`App::with_executor`](crate::tui::app::App::with_executor) to control time and
//! blocking jobs in tests. Advancing time or running a job does not step the
//! application automatically.
//!
//! # Example
//!
//! ```
//! use rsille::tui::{app::App, widgets::label};
//!
//! let mut runtime = App::new(0_u32)
//!     .build(|count, increment| *count += increment, |count| label(count.to_string()))
//!     .size(20, 3);
//!
//! runtime.handle().send(2).unwrap();
//! let result = runtime.step(&[]).unwrap();
//! assert!(result.rendered);
//! assert_eq!(*runtime.state(), 2);
//! assert_eq!(runtime.frame().size().width, 20);
//! ```

mod core;
pub use core::{AppHandle, Runtime, StepResult};
pub(crate) use core::{AppRuntime, SimpleUpdate, UpdateDriver};

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Monotonic time used by updates, timers, task deadlines and animations.
///
/// Successive readings must not move backwards. All runtime deadlines use this
/// clock's time domain; custom clocks need not advance with wall-clock time.
pub trait Clock: Send + Sync {
    /// Returns the current instant in this clock's time domain.
    fn now(&self) -> Instant;
}

/// The default clock, backed by [`Instant::now`].
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A clock advanced explicitly by tests or an embedding host.
///
/// Clones share the same instant. The default value starts at [`Instant::now`].
/// After advancing, call [`Runtime::step`] to process the new time; advancing
/// alone does not wake the runtime.
#[derive(Clone, Debug)]
pub struct ManualClock(Arc<Mutex<Instant>>);

impl Default for ManualClock {
    fn default() -> Self {
        Self::new(Instant::now())
    }
}

impl ManualClock {
    /// Creates a clock fixed at `now` until explicitly advanced.
    pub fn new(now: Instant) -> Self {
        Self(Arc::new(Mutex::new(now)))
    }

    /// Moves the shared instant forward by `duration` without sleeping.
    ///
    /// # Panics
    ///
    /// Panics if the new instant cannot be represented or the clock's mutex
    /// has been poisoned.
    pub fn advance(&self, duration: Duration) {
        *self.0.lock().expect("clock poisoned") += duration;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        *self.0.lock().expect("clock poisoned")
    }
}

/// An owned blocking job submitted to an [`Executor`] for one execution.
pub type Job = Box<dyn FnOnce() + Send + 'static>;

/// Execution boundary for blocking tasks and subscription producers.
///
/// Implementations must schedule work without blocking the UI thread. Jobs may
/// be long-lived and should cooperate with their task's cancellation token.
pub trait Executor: Send + Sync {
    /// Schedules `job` for execution without waiting for it to finish.
    fn spawn(&self, job: Job);
}

/// The default executor, spawning a new detached OS thread for each job.
///
/// This executor does not pool threads or join them during runtime shutdown.
#[derive(Clone, Copy, Debug, Default)]
pub struct ThreadExecutor;

impl Executor for ThreadExecutor {
    fn spawn(&self, job: Job) {
        std::thread::spawn(job);
    }
}

/// Queues jobs for explicit execution on the calling thread.
///
/// Clones share a FIFO queue. [`run_next`](Self::run_next) runs one job to
/// completion, so long-lived producers can block the caller. After a job emits
/// messages, call [`Runtime::step`] to process them.
///
/// # Example
///
/// ```
/// use rsille::tui::runtime::{Executor, ManualExecutor};
///
/// let executor = ManualExecutor::default();
/// executor.spawn(Box::new(|| {}));
/// assert_eq!(executor.pending(), 1);
/// assert!(executor.run_next());
/// assert!(!executor.run_next());
/// ```
#[derive(Clone, Default)]
pub struct ManualExecutor(Arc<Mutex<VecDeque<Job>>>);

impl ManualExecutor {
    /// Removes and runs the oldest queued job, returning whether a job ran.
    ///
    /// The queue lock is released before invoking the job, allowing the job to
    /// enqueue more work. Returns `false` if the queue is empty.
    ///
    /// # Panics
    ///
    /// Panics if the queue's mutex is poisoned or the job itself panics.
    pub fn run_next(&self) -> bool {
        let job = self.0.lock().expect("executor poisoned").pop_front();
        if let Some(job) = job {
            job();
            true
        } else {
            false
        }
    }

    /// Returns the number of queued jobs, excluding any currently executing job.
    ///
    /// # Panics
    ///
    /// Panics if the queue's mutex has been poisoned.
    pub fn pending(&self) -> usize {
        self.0.lock().expect("executor poisoned").len()
    }
}

impl Executor for ManualExecutor {
    fn spawn(&self, job: Job) {
        self.0.lock().expect("executor poisoned").push_back(job);
    }
}
