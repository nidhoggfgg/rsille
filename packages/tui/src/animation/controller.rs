//! Animation controller for managing multiple animations

use super::Animation;
use std::collections::HashMap;
use std::time::Instant;

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
            self.states
                .insert(name.to_string(), AnimationState::Running);
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
                self.states
                    .insert(name.to_string(), AnimationState::Running);
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
        self.states
            .get(name)
            .copied()
            .unwrap_or(AnimationState::Idle)
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
