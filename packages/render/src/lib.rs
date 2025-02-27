mod builder;
mod render;
mod traits;

pub use builder::Builder;
pub use render::*;
pub use traits::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DrawErr;
