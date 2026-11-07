//! Input and action widgets.
//!
//! Use [`button::Button`] for actions, [`checkbox::Checkbox`] or [`switch::Switch`]
//! for booleans, and [`text_input::TextInput`] or [`textarea::TextArea`] for editable
//! text. These value-based controls emit proposals through callbacks; store the
//! message payload in application state and supply it on the next view rebuild.
//! [`calendar::Calendar`] and [`select::Select`] keep navigation state internally.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::widgets::{checkbox, text_input};
//!
//! enum Message {
//!     NameChanged(String),
//!     EnabledChanged(bool),
//! }
//!
//! let name = String::from("Ada");
//! let enabled = true;
//! let input = text_input().key("name").value(name).on_change(Message::NameChanged);
//! let toggle = checkbox("Enabled").checked(enabled).on_change(Message::EnabledChanged);
//! # let _ = (input, toggle);
//! ```

pub mod button;
pub mod calendar;
pub mod checkbox;
pub mod radio_group;
pub mod select;
pub mod switch;
pub mod text_input;
pub mod textarea;
