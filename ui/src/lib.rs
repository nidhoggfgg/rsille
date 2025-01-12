pub mod attr;
pub mod event;
pub mod interactive;
pub mod panel;
pub mod reactive;
pub mod runtime;
pub mod slot;
pub mod style;
pub mod traits;

pub use traits::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DrawErr;
