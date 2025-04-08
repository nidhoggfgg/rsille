mod builder;
mod draw_update;
mod event_loop;
pub mod style;

use std::{error, fmt, io};

pub use builder::Builder;
pub use draw_update::*;
pub use event_loop::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DrawErr;

impl From<DrawErr> for io::Error {
    fn from(value: DrawErr) -> Self {
        io::Error::new(io::ErrorKind::Other, value)
    }
}

impl fmt::Display for DrawErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("draw error")
    }
}

impl error::Error for DrawErr {}
