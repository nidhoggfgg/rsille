//! Terminal-independent runtime support. See [`crate::app::App::build`].

mod core;
pub use core::{AppHandle, Runtime, StepResult};
pub(crate) use core::{AppRuntime, SimpleUpdate, UpdateDriver};

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Monotonic time used by updates, timers, task deadlines and animations.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A clock advanced explicitly by tests or an embedding host.
/// After advancing, call `Runtime::step` to process the new time.
#[derive(Clone, Debug)]
pub struct ManualClock(Arc<Mutex<Instant>>);

impl Default for ManualClock {
    fn default() -> Self {
        Self::new(Instant::now())
    }
}

impl ManualClock {
    pub fn new(now: Instant) -> Self {
        Self(Arc::new(Mutex::new(now)))
    }

    pub fn advance(&self, duration: Duration) {
        *self.0.lock().expect("clock poisoned") += duration;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        *self.0.lock().expect("clock poisoned")
    }
}

pub type Job = Box<dyn FnOnce() + Send + 'static>;

/// Execution boundary for the existing blocking task API.
/// Implementations must enqueue work without blocking the UI thread.
pub trait Executor: Send + Sync {
    fn spawn(&self, job: Job);
}

/// Default execution policy for existing blocking tasks.
#[derive(Clone, Copy, Debug, Default)]
pub struct ThreadExecutor;

impl Executor for ThreadExecutor {
    fn spawn(&self, job: Job) {
        std::thread::spawn(job);
    }
}

/// Queues jobs until the test explicitly runs them; no threads or sleeps.
#[derive(Clone, Default)]
pub struct ManualExecutor(Arc<Mutex<VecDeque<Job>>>);

impl ManualExecutor {
    pub fn run_next(&self) -> bool {
        let job = self.0.lock().expect("executor poisoned").pop_front();
        if let Some(job) = job {
            job();
            true
        } else {
            false
        }
    }

    pub fn pending(&self) -> usize {
        self.0.lock().expect("executor poisoned").len()
    }
}

impl Executor for ManualExecutor {
    fn spawn(&self, job: Job) {
        self.0.lock().expect("executor poisoned").push_back(job);
    }
}
