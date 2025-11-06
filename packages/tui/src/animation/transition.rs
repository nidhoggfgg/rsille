//! Transition system for animating property changes

use super::{Animation, Interpolate, ColorInterpolate};
use crate::style::Color;
use std::time::Duration;

/// Types of transitions available
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionType {
    /// Fade in/out (opacity)
    Fade,
    /// Slide in from direction
    Slide,
    /// Scale up/down
    Scale,
    /// Color transition
    Color,
}

/// Animated value that can transition between states
#[derive(Debug, Clone)]
pub enum AnimatedValue<T> {
    /// Static value (no animation)
    Static(T),
    /// Animated value with from/to and animation
    Animated {
        from: T,
        to: T,
        animation: Animation,
    },
}

impl<T: Interpolate + Clone> AnimatedValue<T> {
    /// Create a new static value
    pub fn new(value: T) -> Self {
        Self::Static(value)
    }

    /// Create an animated transition
    pub fn transition(from: T, to: T, duration: Duration) -> Self {
        Self::Animated {
            from,
            to,
            animation: Animation::new(duration),
        }
    }

    /// Start the animation
    pub fn start(&mut self) {
        if let Self::Animated { animation, .. } = self {
            animation.start();
        }
    }

    /// Get current value based on animation progress
    pub fn current(&self) -> T {
        match self {
            Self::Static(value) => value.clone(),
            Self::Animated { from, to, animation } => {
                let t = animation.eased_progress();
                from.interpolate(to, t)
            }
        }
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        match self {
            Self::Static(_) => true,
            Self::Animated { animation, .. } => animation.is_complete(),
        }
    }

    /// Set the value (converting to static)
    pub fn set(&mut self, value: T) {
        *self = Self::Static(value);
    }
}

/// Opacity transition helper (0.0 to 1.0)
#[derive(Debug, Clone)]
pub struct OpacityTransition {
    value: AnimatedValue<f32>,
}

impl OpacityTransition {
    /// Create a new opacity at full visibility
    pub fn new() -> Self {
        Self {
            value: AnimatedValue::new(1.0),
        }
    }

    /// Create with initial opacity
    pub fn with_opacity(opacity: f32) -> Self {
        Self {
            value: AnimatedValue::new(opacity.clamp(0.0, 1.0)),
        }
    }

    /// Fade in from 0 to 1
    pub fn fade_in(&mut self, duration: Duration) {
        self.value = AnimatedValue::transition(0.0, 1.0, duration);
        self.value.start();
    }

    /// Fade out from 1 to 0
    pub fn fade_out(&mut self, duration: Duration) {
        self.value = AnimatedValue::transition(1.0, 0.0, duration);
        self.value.start();
    }

    /// Fade from current to target opacity
    pub fn fade_to(&mut self, target: f32, duration: Duration) {
        let current = self.value.current();
        self.value = AnimatedValue::transition(current, target.clamp(0.0, 1.0), duration);
        self.value.start();
    }

    /// Get current opacity
    pub fn opacity(&self) -> f32 {
        self.value.current()
    }

    /// Get alpha as u8 (0-255)
    pub fn alpha_u8(&self) -> u8 {
        (self.opacity() * 255.0) as u8
    }

    /// Check if fully visible
    pub fn is_visible(&self) -> bool {
        self.opacity() > 0.01
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        self.value.is_complete()
    }
}

impl Default for OpacityTransition {
    fn default() -> Self {
        Self::new()
    }
}

/// Color transition helper
#[derive(Debug, Clone)]
pub struct ColorTransition {
    from: Color,
    to: Color,
    animation: Animation,
    active: bool,
}

impl ColorTransition {
    /// Create a new color transition
    pub fn new(initial_color: Color) -> Self {
        Self {
            from: initial_color,
            to: initial_color,
            animation: Animation::new(Duration::from_millis(300)),
            active: false,
        }
    }

    /// Transition to a new color
    pub fn transition_to(&mut self, color: Color, duration: Duration) {
        self.from = self.current();
        self.to = color;
        self.animation = Animation::new(duration);
        self.animation.start();
        self.active = true;
    }

    /// Get current color
    pub fn current(&self) -> Color {
        if !self.active {
            return self.to;
        }

        let t = self.animation.eased_progress();
        Color::interpolate_color(&self.from, &self.to, t)
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        !self.active || self.animation.is_complete()
    }

    /// Update - returns true if still animating
    pub fn update(&mut self) -> bool {
        if self.active && self.animation.is_complete() {
            self.active = false;
            self.from = self.to;
        }
        self.active
    }
}

/// Position transition for sliding effects
#[derive(Debug, Clone)]
pub struct PositionTransition {
    x: AnimatedValue<f32>,
    y: AnimatedValue<f32>,
}

impl PositionTransition {
    /// Create at origin
    pub fn new() -> Self {
        Self {
            x: AnimatedValue::new(0.0),
            y: AnimatedValue::new(0.0),
        }
    }

    /// Create at specific position
    pub fn at(x: f32, y: f32) -> Self {
        Self {
            x: AnimatedValue::new(x),
            y: AnimatedValue::new(y),
        }
    }

    /// Slide to position
    pub fn slide_to(&mut self, x: f32, y: f32, duration: Duration) {
        let current_x = self.x.current();
        let current_y = self.y.current();

        self.x = AnimatedValue::transition(current_x, x, duration);
        self.y = AnimatedValue::transition(current_y, y, duration);

        self.x.start();
        self.y.start();
    }

    /// Slide from offscreen (left)
    pub fn slide_in_left(&mut self, target_x: f32, target_y: f32, duration: Duration) {
        self.x = AnimatedValue::transition(-100.0, target_x, duration);
        self.y = AnimatedValue::new(target_y);
        self.x.start();
    }

    /// Slide from offscreen (right)
    pub fn slide_in_right(&mut self, target_x: f32, target_y: f32, duration: Duration) {
        self.x = AnimatedValue::transition(100.0, target_x, duration);
        self.y = AnimatedValue::new(target_y);
        self.x.start();
    }

    /// Slide from offscreen (top)
    pub fn slide_in_top(&mut self, target_x: f32, target_y: f32, duration: Duration) {
        self.x = AnimatedValue::new(target_x);
        self.y = AnimatedValue::transition(-100.0, target_y, duration);
        self.y.start();
    }

    /// Slide from offscreen (bottom)
    pub fn slide_in_bottom(&mut self, target_x: f32, target_y: f32, duration: Duration) {
        self.x = AnimatedValue::new(target_x);
        self.y = AnimatedValue::transition(100.0, target_y, duration);
        self.y.start();
    }

    /// Get current position
    pub fn position(&self) -> (f32, f32) {
        (self.x.current(), self.y.current())
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        self.x.is_complete() && self.y.is_complete()
    }
}

impl Default for PositionTransition {
    fn default() -> Self {
        Self::new()
    }
}

/// Scale transition for zoom effects
#[derive(Debug, Clone)]
pub struct ScaleTransition {
    scale: AnimatedValue<f32>,
}

impl ScaleTransition {
    /// Create at normal scale (1.0)
    pub fn new() -> Self {
        Self {
            scale: AnimatedValue::new(1.0),
        }
    }

    /// Create at specific scale
    pub fn with_scale(scale: f32) -> Self {
        Self {
            scale: AnimatedValue::new(scale),
        }
    }

    /// Scale to target
    pub fn scale_to(&mut self, target: f32, duration: Duration) {
        let current = self.scale.current();
        self.scale = AnimatedValue::transition(current, target, duration);
        self.scale.start();
    }

    /// Scale up from 0
    pub fn scale_in(&mut self, duration: Duration) {
        self.scale = AnimatedValue::transition(0.0, 1.0, duration);
        self.scale.start();
    }

    /// Scale down to 0
    pub fn scale_out(&mut self, duration: Duration) {
        let current = self.scale.current();
        self.scale = AnimatedValue::transition(current, 0.0, duration);
        self.scale.start();
    }

    /// Get current scale
    pub fn scale(&self) -> f32 {
        self.scale.current()
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        self.scale.is_complete()
    }
}

impl Default for ScaleTransition {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined transition for complex animations
#[derive(Debug, Clone)]
pub struct Transition {
    pub opacity: OpacityTransition,
    pub position: PositionTransition,
    pub scale: ScaleTransition,
}

impl Transition {
    /// Create a new transition with defaults
    pub fn new() -> Self {
        Self {
            opacity: OpacityTransition::new(),
            position: PositionTransition::new(),
            scale: ScaleTransition::new(),
        }
    }

    /// Check if any animation is active
    pub fn is_animating(&self) -> bool {
        !self.opacity.is_complete()
            || !self.position.is_complete()
            || !self.scale.is_complete()
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opacity_transition() {
        let mut opacity = OpacityTransition::new();
        assert_eq!(opacity.opacity(), 1.0);

        opacity.fade_out(Duration::from_millis(100));
        // Sleep a bit to allow animation to progress
        std::thread::sleep(Duration::from_millis(10));
        let current = opacity.opacity();
        // Should be less than 1.0 but greater than 0.0
        assert!(current < 1.0 && current > 0.0, "Opacity should be animating: {}", current);
    }

    #[test]
    fn test_position_transition() {
        let mut pos = PositionTransition::new();
        assert_eq!(pos.position(), (0.0, 0.0));

        pos.slide_to(10.0, 20.0, Duration::from_millis(100));
        let (x, y) = pos.position();
        // Should be animating towards target
        assert!(x >= 0.0 && x <= 10.0);
        assert!(y >= 0.0 && y <= 20.0);
    }

    #[test]
    fn test_scale_transition() {
        let mut scale = ScaleTransition::new();
        assert_eq!(scale.scale(), 1.0);

        scale.scale_to(2.0, Duration::from_millis(100));
        let s = scale.scale();
        // Should be animating towards target
        assert!(s >= 1.0 && s <= 2.0);
    }
}
