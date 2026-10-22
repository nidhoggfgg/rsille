pub mod braille;
pub mod utils;

pub use canvas_impl::Canvas;
pub use canvas_impl::Paint;
pub use canvas_impl::PaintErr;

pub mod color;

mod bound;
#[path = "canvas.rs"]
mod canvas_impl;
mod tile;
