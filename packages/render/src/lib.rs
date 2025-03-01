mod builder;
mod draw_update;
mod render;
pub mod style;

use std::io;

pub use builder::Builder;
pub use draw_update::*;
pub use render::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DrawErr;

impl Into<io::Error> for DrawErr {
    fn into(self) -> io::Error {
        io::Error::new(io::ErrorKind::Other, "")
    }
}
