//! Easing functions for smooth animations
//!
//! Based on common easing functions used in CSS and animation libraries.
//! See: https://easings.net/

/// Easing function types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    /// Linear interpolation (no easing)
    Linear,

    // Quadratic
    /// Accelerating from zero velocity
    EaseInQuad,
    /// Decelerating to zero velocity
    EaseOutQuad,
    /// Acceleration until halfway, then deceleration
    EaseInOutQuad,

    // Cubic
    /// Accelerating from zero velocity (cubic)
    EaseInCubic,
    /// Decelerating to zero velocity (cubic)
    EaseOutCubic,
    /// Acceleration until halfway, then deceleration (cubic)
    EaseInOutCubic,

    // Quartic
    /// Accelerating from zero velocity (quartic)
    EaseInQuart,
    /// Decelerating to zero velocity (quartic)
    EaseOutQuart,
    /// Acceleration until halfway, then deceleration (quartic)
    EaseInOutQuart,

    // Exponential
    /// Accelerating from zero velocity (exponential)
    EaseInExpo,
    /// Decelerating to zero velocity (exponential)
    EaseOutExpo,
    /// Acceleration until halfway, then deceleration (exponential)
    EaseInOutExpo,

    // Circular
    /// Accelerating from zero velocity (circular)
    EaseInCirc,
    /// Decelerating to zero velocity (circular)
    EaseOutCirc,
    /// Acceleration until halfway, then deceleration (circular)
    EaseInOutCirc,

    // Back
    /// Back easing in - overshoots and returns
    EaseInBack,
    /// Back easing out - overshoots and settles
    EaseOutBack,
    /// Back easing in-out
    EaseInOutBack,

    // Elastic
    /// Elastic bounce effect (ease in)
    EaseInElastic,
    /// Elastic bounce effect (ease out)
    EaseOutElastic,
    /// Elastic bounce effect (ease in-out)
    EaseInOutElastic,

    // Bounce
    /// Bounce effect (ease in)
    EaseInBounce,
    /// Bounce effect (ease out)
    EaseOutBounce,
    /// Bounce effect (ease in-out)
    EaseInOutBounce,
}

impl Easing {
    /// Apply the easing function to a progress value (0.0 to 1.0)
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);

        match self {
            Easing::Linear => t,

            // Quadratic
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => t * (2.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }

            // Cubic
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => {
                let t = t - 1.0;
                t * t * t + 1.0
            }
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let t = 2.0 * t - 2.0;
                    (t * t * t + 2.0) / 2.0
                }
            }

            // Quartic
            Easing::EaseInQuart => t * t * t * t,
            Easing::EaseOutQuart => {
                let t = t - 1.0;
                1.0 - t * t * t * t
            }
            Easing::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    let t = t - 1.0;
                    1.0 - 8.0 * t * t * t * t
                }
            }

            // Exponential
            Easing::EaseInExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0_f32.powf(10.0 * (t - 1.0))
                }
            }
            Easing::EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f32.powf(-10.0 * t)
                }
            }
            Easing::EaseInOutExpo => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2.0_f32.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
                }
            }

            // Circular
            Easing::EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
            Easing::EaseOutCirc => {
                let t = t - 1.0;
                (1.0 - t * t).sqrt()
            }
            Easing::EaseInOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - 4.0 * t * t).sqrt()) / 2.0
                } else {
                    let t = -2.0 * t + 2.0;
                    ((1.0 - t * t).sqrt() + 1.0) / 2.0
                }
            }

            // Back
            Easing::EaseInBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                C3 * t * t * t - C1 * t * t
            }
            Easing::EaseOutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                let t = t - 1.0;
                1.0 + C3 * t * t * t + C1 * t * t
            }
            Easing::EaseInOutBack => {
                const C1: f32 = 1.70158;
                const C2: f32 = C1 * 1.525;
                if t < 0.5 {
                    let t = 2.0 * t;
                    (t * t * ((C2 + 1.0) * t - C2)) / 2.0
                } else {
                    let t = 2.0 * t - 2.0;
                    (t * t * ((C2 + 1.0) * t + C2) + 2.0) / 2.0
                }
            }

            // Elastic
            Easing::EaseInElastic => {
                const C4: f32 = (2.0 * std::f32::consts::PI) / 3.0;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    -2.0_f32.powf(10.0 * t - 10.0) * ((t * 10.0 - 10.75) * C4).sin()
                }
            }
            Easing::EaseOutElastic => {
                const C4: f32 = (2.0 * std::f32::consts::PI) / 3.0;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
                }
            }
            Easing::EaseInOutElastic => {
                const C5: f32 = (2.0 * std::f32::consts::PI) / 4.5;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    -(2.0_f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * C5).sin()) / 2.0
                } else {
                    (2.0_f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * C5).sin()) / 2.0
                        + 1.0
                }
            }

            // Bounce
            Easing::EaseInBounce => 1.0 - Self::EaseOutBounce.apply(1.0 - t),
            Easing::EaseOutBounce => {
                const N1: f32 = 7.5625;
                const D1: f32 = 2.75;

                if t < 1.0 / D1 {
                    N1 * t * t
                } else if t < 2.0 / D1 {
                    let t = t - 1.5 / D1;
                    N1 * t * t + 0.75
                } else if t < 2.5 / D1 {
                    let t = t - 2.25 / D1;
                    N1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / D1;
                    N1 * t * t + 0.984375
                }
            }
            Easing::EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - Self::EaseOutBounce.apply(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + Self::EaseOutBounce.apply(2.0 * t - 1.0)) / 2.0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear() {
        assert_eq!(Easing::Linear.apply(0.0), 0.0);
        assert_eq!(Easing::Linear.apply(0.5), 0.5);
        assert_eq!(Easing::Linear.apply(1.0), 1.0);
    }

    #[test]
    fn test_ease_in_quad() {
        assert_eq!(Easing::EaseInQuad.apply(0.0), 0.0);
        assert_eq!(Easing::EaseInQuad.apply(1.0), 1.0);
        // Should be slower at start
        assert!(Easing::EaseInQuad.apply(0.5) < 0.5);
    }

    #[test]
    fn test_ease_out_quad() {
        assert_eq!(Easing::EaseOutQuad.apply(0.0), 0.0);
        assert_eq!(Easing::EaseOutQuad.apply(1.0), 1.0);
        // Should be faster at start
        assert!(Easing::EaseOutQuad.apply(0.5) > 0.5);
    }

    #[test]
    fn test_clamp() {
        // Values outside 0-1 should be clamped
        assert_eq!(Easing::Linear.apply(-0.5), 0.0);
        assert_eq!(Easing::Linear.apply(1.5), 1.0);
    }
}
