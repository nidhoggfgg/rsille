//! Animation and visual effect widgets.
//!
//! [`animated::Animated`] declares layout, shared-element, and enter/exit
//! transitions around a child. [`visual::Visual`] additionally applies cell-level
//! effects. Configure an animation or an explicit progress value to drive a
//! visual effect; progress is independent of the child's application messages.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::animation::AnimationSpec;
//! use rsille::tui::widgets::{animate, label, visual};
//!
//! let moving = animate(label::<()>("Moving")).layout(AnimationSpec::normal());
//! let fading = visual(label::<()>("Appearing")).fade_in().progress(0.5);
//! # let _ = (moving, fading);
//! ```

pub mod animated;
pub mod visual;
