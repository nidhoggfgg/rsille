//! Effect runtime for executing effects asynchronously

use super::Effect;
use std::sync::{Arc, Mutex};
use std::thread;

/// Runtime for executing effects and collecting their messages
pub struct EffectRuntime<M> {
    message_queue: Arc<Mutex<Vec<M>>>,
}

impl<M: Send + 'static> EffectRuntime<M> {
    /// Create a new effect runtime
    pub fn new() -> Self {
        Self {
            message_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Execute an effect asynchronously
    pub fn execute(&self, effect: Effect<M>) {
        match effect {
            Effect::Delay { duration, message } => {
                let queue = self.message_queue.clone();
                thread::spawn(move || {
                    thread::sleep(duration);
                    if let Ok(mut messages) = queue.lock() {
                        messages.push(message);
                    }
                });
            }
            Effect::Task { task } => {
                let queue = self.message_queue.clone();
                thread::spawn(move || {
                    let message = task();
                    if let Ok(mut messages) = queue.lock() {
                        messages.push(message);
                    }
                });
            }
            Effect::Batch(effects) => {
                for effect in effects {
                    self.execute(effect);
                }
            }
            Effect::None => {}
        }
    }

    /// Collect all messages that have been produced by effects
    pub fn collect_messages(&self) -> Vec<M> {
        if let Ok(mut messages) = self.message_queue.lock() {
            std::mem::take(&mut *messages)
        } else {
            Vec::new()
        }
    }

    /// Check if there are any pending messages
    pub fn has_messages(&self) -> bool {
        if let Ok(messages) = self.message_queue.lock() {
            !messages.is_empty()
        } else {
            false
        }
    }
}

impl<M: Send + 'static> Default for EffectRuntime<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> std::fmt::Debug for EffectRuntime<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EffectRuntime {{ ... }}")
    }
}
