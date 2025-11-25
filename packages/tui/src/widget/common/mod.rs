//! Common utilities for widget implementation
//!
//! This module provides shared abstractions and utilities that reduce code
//! duplication across different widget implementations.
//!
//! # Modules
//!
//! - `state` - Common widget state management (`WidgetState`)
//! - `style_manager` - Style computation helpers (`StyleManager`)
//! - `navigation` - Navigation utilities for selectable widgets (`SelectableNavigation`)
//!
//! # Examples
//!
//! Using `WidgetState` to manage common widget state:
//! ```
//! use tui::widget::common::WidgetState;
//!
//! let state = WidgetState::new()
//!     .with_disabled(false);
//!
//! assert!(state.is_focusable());
//! ```
//!
//! Using `StyleManager` to compute widget styles:
//! ```
//! use tui::widget::common::{WidgetState, StyleManager};
//!
//! let state = WidgetState::new();
//! let style = StyleManager::interactive_style(&state);
//! ```
//!
//! Using `SelectableNavigation` for list-like widgets:
//! ```
//! use tui::widget::common::SelectableNavigation;
//!
//! let mut nav = SelectableNavigation::new(10, 5);
//! nav.focus_next(|_| false, 10); // Move to next item
//! ```

mod navigation;
mod state;
mod style_manager;

pub use navigation::SelectableNavigation;
pub use state::WidgetState;
pub use style_manager::StyleManager;
