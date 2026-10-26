pub mod braille;
pub mod utils;

mod color;
mod encoder;
mod encoding;
mod sample;

pub use encoding::Encoding;

pub use canvas_impl::Canvas;
pub use canvas_impl::Paint;
pub use canvas_impl::PaintErr;

mod bound;
#[path = "canvas.rs"]
mod canvas_impl;
mod tile;

#[cfg(test)]
mod tests;
