//! Read-only display and spacing widgets.
//!
//! [`label::Label`] handles text and wrapping; [`content`] provides scrollable
//! viewers for code, logs, diffs, and Markdown. [`progress`] displays progress
//! fractions or application-driven spinner frames. [`canvas::CanvasWidget`]
//! renders sample graphics, with [`world_canvas`] providing clipped drawing in
//! world coordinates. [`divider`] and [`spacer`] separate content visually.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::widgets::{label, progress_bar, CanvasPoint, CanvasViewport};
//!
//! let title = label::<()>("Downloading").bold().wrap_word();
//! let progress = progress_bar::<()>(0.5).label("50%");
//! let viewport = CanvasViewport::new(-10.0, -5.0, 20.0, 10.0);
//! assert!(viewport.contains(CanvasPoint::new(0.0, 0.0)));
//! assert_eq!(viewport.pan(2.0, 0.0).x(), -8.0);
//! # let _ = (title, progress);
//! ```

pub mod canvas;
pub mod content;
pub mod divider;
pub mod label;
pub mod progress;
pub mod spacer;

pub mod world_canvas;
