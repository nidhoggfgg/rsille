pub mod braille;
pub mod utils;

pub use canvas::Canvas;
pub use canvas::Paint;
pub use canvas::PaintErr;

pub extern crate crossterm;

#[cfg(feature = "color")]
pub mod color;

mod bound;
mod canvas;
mod tile;
