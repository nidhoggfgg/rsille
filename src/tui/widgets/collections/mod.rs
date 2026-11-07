//! Collection, selection, and hierarchical widgets.
//!
//! Item IDs connect navigation, selection, and callback payloads; keep them unique
//! and stable across view rebuilds. [`selection::SelectionState`] separates the
//! navigation cursor from committed selection. Lists, tables, trees, and file
//! explorers use this shared model.
//!
//! Use [`virtual_list::VirtualList`] with a [`virtual_list::DataSource`] for large
//! collections whose rows should be constructed only near the visible viewport.
//! Its selection and scroll state are owned by the application.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::widgets::{list, ListItem, SelectionMode, SelectionState};
//!
//! let files = list::<Vec<String>>()
//!     .items([ListItem::new("readme", "README.md"), ListItem::new("src", "src/")])
//!     .selection_mode(SelectionMode::Multiple)
//!     .on_selection_change(|ids| ids);
//!
//! let mut selection = SelectionState::default();
//! selection.set_cursor(Some("readme".into()));
//! assert!(selection.toggle("src"));
//! assert_eq!(selection.cursor(), Some("readme"));
//! assert_eq!(selection.primary_selected(), Some("src"));
//! # let _ = files;
//! ```

pub mod data_table;
pub mod file_explorer;
pub mod list;
pub mod selection;
pub mod tree;

pub mod virtual_list;
