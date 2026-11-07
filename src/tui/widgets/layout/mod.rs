//! Layout and container widgets.
//!
//! [`panel::Panel`] and [`dialog::Dialog`] arrange children vertically with borders,
//! padding, and gaps. A dialog also defines a focus scope; place it in an overlay
//! for modal presentation. [`collapsible::Collapsible`] exposes application-owned
//! expansion state. [`drag_region::DragRegion`] recognizes drag gestures, while
//! [`floating_window::FloatingWindow`] provides movable, resizable window chrome.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::widgets::{button, dialog, label};
//!
//! let confirmation = dialog::<()>()
//!     .title("Confirm")
//!     .size(32, 8)
//!     .child(label("Continue with this operation?"))
//!     .child(button("Continue").on_click(|| ()));
//! # let _ = confirmation;
//! ```

pub mod collapsible;
pub mod dialog;
pub mod panel;

pub mod drag_region;

pub mod floating_window;
