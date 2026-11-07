//! Navigation and command widgets.
//!
//! [`tabs::Tabs`] emits tab values; the application chooses which page to render.
//! [`menu::Menu`] emits action IDs and dismissal requests.
//! [`command_palette::CommandPalette`] filters commands by label, ID, and keywords
//! and emits the activated command ID. These widgets do not switch application
//! pages or execute commands themselves.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::widgets::{command_palette, tabs, CommandItem, TabItem};
//!
//! let pages = tabs::<String>()
//!     .tabs([TabItem::new("home", "Home"), TabItem::new("settings", "Settings")])
//!     .selected("home")
//!     .on_change(|value| value);
//! let commands = command_palette::<String>()
//!     .item(CommandItem::new("open", "Open file").keyword("browse"))
//!     .on_submit(|id| id);
//! # let _ = (pages, commands);
//! ```

pub mod command_palette;
pub mod menu;
pub mod tabs;
