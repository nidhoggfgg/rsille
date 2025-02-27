mod builder;
mod engine;
mod traits;

pub use builder::Builder;
pub use engine::*;
pub use traits::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DrawErr;
