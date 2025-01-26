pub mod attr;
pub mod composite;
pub mod engine;

mod style;
mod traits;

pub use style::Stylized;
pub use traits::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DrawErr;
