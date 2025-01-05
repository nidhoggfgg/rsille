pub mod attr;
pub mod panel;
pub mod reactive;
pub mod slot;
pub mod style;
pub mod traits;
pub mod view;

pub extern crate async_trait;
pub use traits::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DrawErr;
