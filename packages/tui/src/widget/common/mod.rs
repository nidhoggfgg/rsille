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
//! - `stateful_builder` - Builder pattern trait for stateful widgets (`StatefulWidgetBuilder`)
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
//!
//! Using `StatefulWidgetBuilder` trait to add builder methods:
//! ```
//! use tui::widget::common::{WidgetState, StatefulWidgetBuilder};
//!
//! struct MyWidget {
//!     state: WidgetState,
//! }
//!
//! impl StatefulWidgetBuilder for MyWidget {
//!     fn widget_state_mut(&mut self) -> &mut WidgetState {
//!         &mut self.state
//!     }
//! }
//! ```

mod navigation;
mod state;
mod stateful_builder;
mod style_manager;

pub use navigation::SelectableNavigation;
pub use state::WidgetState;
pub use stateful_builder::StatefulWidgetBuilder;
pub use style_manager::StyleManager;
