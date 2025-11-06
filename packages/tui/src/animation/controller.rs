//! Animation controller for managing multiple animations

use super::Animation;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Animation state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationState {
    /// Animation is idle (not started)
    Idle,
    /// Animation is running
    Running,
    /// Animation is paused
    Paused,
    /// Animation has completed
    Completed,
}

/// Controller for managing multiple named animations
pub struct AnimationController {
    animations: HashMap<String, Animation>,
    states: HashMap<String, AnimationState>,
    last_update: Instant,
}

impl AnimationController {
    /// Create a new animation controller
    pub fn new() -> Self {
        Self {
            animations: HashMap::new(),
            states: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    /// Register a new animation with a name
    pub fn register(&mut self, name: impl Into<String>, animation: Animation) {
        let name = name.into();
        self.animations.insert(name.clone(), animation);
        self.states.insert(name, AnimationState::Idle);
    }

    /// Start an animation by name
    pub fn start(&mut self, name: &str) -> bool {
        if let Some(animation) = self.animations.get_mut(name) {
            animation.start();
            self.states.insert(name.to_string(), AnimationState::Running);
            true
        } else {
            false
        }
    }

    /// Pause an animation by name
    pub fn pause(&mut self, name: &str) -> bool {
        if self.states.contains_key(name) {
            self.states.insert(name.to_string(), AnimationState::Paused);
            true
        } else {
            false
        }
    }

    /// Resume a paused animation
    pub fn resume(&mut self, name: &str) -> bool {
        if let Some(state) = self.states.get(name) {
            if *state == AnimationState::Paused {
                self.states.insert(name.to_string(), AnimationState::Running);
                return true;
            }
        }
        false
    }

    /// Stop and reset an animation
    pub fn reset(&mut self, name: &str) -> bool {
        if let Some(animation) = self.animations.get_mut(name) {
            animation.reset();
            self.states.insert(name.to_string(), AnimationState::Idle);
            true
        } else {
            false
        }
    }

    /// Get the current progress of an animation (0.0 to 1.0)
    pub fn progress(&self, name: &str) -> Option<f32> {
        self.animations.get(name).map(|a| a.eased_progress())
    }

    /// Get the state of an animation
    pub fn state(&self, name: &str) -> AnimationState {
        self.states.get(name).copied().unwrap_or(AnimationState::Idle)
    }

    /// Update all animations
    pub fn update(&mut self) {
        let now = Instant::now();
        let _delta = now.duration_since(self.last_update);
        self.last_update = now;

        // Update animation states
        let completed: Vec<String> = self
            .animations
            .iter()
            .filter_map(|(name, animation)| {
                if animation.is_complete() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        for name in completed {
            self.states.insert(name, AnimationState::Completed);
        }
    }

    /// Check if any animation is running
    pub fn is_animating(&self) -> bool {
        self.states.values().any(|s| *s == AnimationState::Running)
    }

    /// Get all animation names
    pub fn animation_names(&self) -> Vec<&str> {
        self.animations.keys().map(|s| s.as_str()).collect()
    }

    /// Remove an animation
    pub fn remove(&mut self, name: &str) -> bool {
        let removed_anim = self.animations.remove(name).is_some();
        let removed_state = self.states.remove(name).is_some();
        removed_anim || removed_state
    }

    /// Clear all animations
    pub fn clear(&mut self) {
        self.animations.clear();
        self.states.clear();
    }

    /// Get number of registered animations
    pub fn count(&self) -> usize {
        self.animations.len()
    }
}

impl Default for AnimationController {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper for sequencing animations
pub struct AnimationSequence {
    steps: Vec<(Animation, Option<Duration>)>, // (animation, optional delay before next)
    current_step: usize,
    state: AnimationState,
}

impl AnimationSequence {
    /// Create a new animation sequence
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            current_step: 0,
            state: AnimationState::Idle,
        }
    }

    /// Add an animation to the sequence
    pub fn then(mut self, animation: Animation) -> Self {
        self.steps.push((animation, None));
        self
    }

    /// Add an animation with a delay after it completes
    pub fn then_wait(mut self, animation: Animation, delay: Duration) -> Self {
        self.steps.push((animation, Some(delay)));
        self
    }

    /// Start the sequence
    pub fn start(&mut self) {
        if !self.steps.is_empty() {
            self.current_step = 0;
            self.steps[0].0.start();
            self.state = AnimationState::Running;
        }
    }

    /// Update the sequence - returns true if still running
    pub fn update(&mut self) -> bool {
        if self.state != AnimationState::Running {
            return false;
        }

        if self.current_step >= self.steps.len() {
            self.state = AnimationState::Completed;
            return false;
        }

        let (current_anim, delay) = &mut self.steps[self.current_step];

        if current_anim.is_complete() {
            // Check if there's a delay
            if let Some(_delay) = delay {
                // TODO: Implement delay handling
                // For now, just move to next step
            }

            // Move to next step
            self.current_step += 1;

            if self.current_step < self.steps.len() {
                self.steps[self.current_step].0.start();
            } else {
                self.state = AnimationState::Completed;
                return false;
            }
        }

        true
    }

    /// Get current progress (0.0 to 1.0) across entire sequence
    pub fn progress(&self) -> f32 {
        if self.steps.is_empty() {
            return 1.0;
        }

        let steps_completed = self.current_step as f32;
        let total_steps = self.steps.len() as f32;

        if self.current_step < self.steps.len() {
            let current_progress = self.steps[self.current_step].0.eased_progress();
            (steps_completed + current_progress) / total_steps
        } else {
            1.0
        }
    }

    /// Check if sequence is complete
    pub fn is_complete(&self) -> bool {
        self.state == AnimationState::Completed
    }

    /// Get current state
    pub fn state(&self) -> AnimationState {
        self.state
    }
}

impl Default for AnimationSequence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_basic() {
        let mut controller = AnimationController::new();
        let animation = Animation::new(Duration::from_millis(100));

        controller.register("test", animation);
        assert_eq!(controller.count(), 1);
        assert_eq!(controller.state("test"), AnimationState::Idle);

        controller.start("test");
        assert_eq!(controller.state("test"), AnimationState::Running);
    }

    #[test]
    fn test_sequence() {
        let mut seq = AnimationSequence::new();
        seq = seq
            .then(Animation::new(Duration::from_millis(10)))
            .then(Animation::new(Duration::from_millis(10)));

        seq.start();
        assert_eq!(seq.state(), AnimationState::Running);
        assert!(!seq.is_complete());
    }
}
