//! Animation system for TUI widgets
//!
//! Provides frontend-like animation capabilities including:
//! - Easing functions (ease-in, ease-out, ease-in-out, etc.)
//! - Property interpolation (opacity, position, size, color)
//! - Animation composition and sequencing
//! - Spring-based physics animations
//!
//! # Example
//!
//! ```rust,ignore
//! let animation = Animation::new(Duration::from_millis(300))
//!     .with_easing(Easing::EaseOut)
//!     .fade_in();
//! ```

mod easing;
mod interpolate;
mod transition;
mod controller;

pub use easing::Easing;
pub use interpolate::{Interpolate, ColorInterpolate};
pub use transition::{
    Transition, TransitionType, AnimatedValue, OpacityTransition,
    ColorTransition, PositionTransition, ScaleTransition,
};
pub use controller::{AnimationController, AnimationState};

use std::time::{Duration, Instant};

/// Core animation structure
#[derive(Debug, Clone)]
pub struct Animation {
    /// Animation duration
    pub duration: Duration,
    /// Easing function to use
    pub easing: Easing,
    /// Start time (None if not started)
    pub start_time: Option<Instant>,
    /// Whether animation should loop
    pub looping: bool,
    /// Whether animation plays in reverse after completion
    pub alternate: bool,
    /// Current direction (true = forward)
    direction: bool,
}

impl Animation {
    /// Create a new animation with specified duration
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            easing: Easing::Linear,
            start_time: None,
            looping: false,
            alternate: false,
            direction: true,
        }
    }

    /// Set the easing function
    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Make animation loop infinitely
    pub fn looping(mut self) -> Self {
        self.looping = true;
        self
    }

    /// Make animation alternate direction (ping-pong)
    pub fn alternate(mut self) -> Self {
        self.alternate = true;
        self.looping = true; // Alternate implies looping
        self
    }

    /// Start the animation
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.direction = true;
    }

    /// Get current progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        let Some(start_time) = self.start_time else {
            return 0.0;
        };

        let elapsed = start_time.elapsed();
        let progress = elapsed.as_secs_f32() / self.duration.as_secs_f32();

        if self.looping {
            if self.alternate {
                // Ping-pong animation
                let cycle = progress % 2.0;
                if cycle > 1.0 {
                    1.0 - (cycle - 1.0)
                } else {
                    cycle
                }
            } else {
                // Simple loop
                progress % 1.0
            }
        } else {
            progress.min(1.0)
        }
    }

    /// Get eased progress value
    pub fn eased_progress(&self) -> f32 {
        let progress = self.progress();
        self.easing.apply(progress)
    }

    /// Check if animation is complete (always false for looping animations)
    pub fn is_complete(&self) -> bool {
        if self.looping {
            return false;
        }

        let Some(start_time) = self.start_time else {
            return false;
        };

        start_time.elapsed() >= self.duration
    }

    /// Reset animation to beginning
    pub fn reset(&mut self) {
        self.start_time = None;
        self.direction = true;
    }
}

/// Helper macro for creating animations
#[macro_export]
macro_rules! animate {
    ($duration:expr) => {
        Animation::new($duration)
    };
    ($duration:expr, $easing:expr) => {
        Animation::new($duration).with_easing($easing)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_progress() {
        let mut anim = Animation::new(Duration::from_millis(100));
        assert_eq!(anim.progress(), 0.0);

        anim.start();
        std::thread::sleep(Duration::from_millis(50));
        let progress = anim.progress();
        assert!(progress > 0.4 && progress < 0.6, "Progress should be around 0.5");
    }

    #[test]
    fn test_animation_complete() {
        let mut anim = Animation::new(Duration::from_millis(10));
        assert!(!anim.is_complete());

        anim.start();
        std::thread::sleep(Duration::from_millis(20));
        assert!(anim.is_complete());
    }

    #[test]
    fn test_looping_animation() {
        let mut anim = Animation::new(Duration::from_millis(10)).looping();
        anim.start();
        std::thread::sleep(Duration::from_millis(25));
        assert!(!anim.is_complete()); // Should never complete
    }
}
