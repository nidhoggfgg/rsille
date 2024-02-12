#![warn(missing_docs)]
#![allow(clippy::needless_doctest_main)]
//! This crate is a rust lib for making [braille] art.
//!
//! You can use the basic canvas to paint something,
//! or you can use [turtle] to paint things like in python.
//! And something like colorful output, 3D object and more!
//!
//! ## Examples
//!
//! draw the sin(x)
//! ```
//! use rsille::Canvas;
//! let mut c = Canvas::new();
//! for x in 0..1800 {
//!     let x = x as f64;
//!     c.set(x / 10.0, 15.0 + x.to_radians().sin() * 10.0);
//! }
//! println!("{}", c.frame());
//! ```
//!
//! draw a star
//! ```
//! use rsille::{Turtle, Canvas};
//! let mut c = Canvas::new();
//! let mut t = Turtle::new();
//! for _ in 0..5 {
//!     t.forward(100.0);
//!     t.right(144.0);
//! }
//! c.paint(&t, 0.0, 30.0);
//! println!("{}", c.frame());
//! ```
//!
//! It's inspired by [drawille], but it has more features and fast
//!
//! [braille]: http://www.alanwood.net/unicode/braille_patterns.html
//! [turtle]: https://docs.python.org/3/library/turtle.html
//! [drawille]: https://github.com/asciimoo/drawille

mod braille;
mod canvas;
mod defaults;
pub mod object3d;
mod turtle;
mod utils;

pub use canvas::Canvas;
pub use canvas::Paint;
pub use turtle::Turtle;

#[cfg(feature = "color")]
pub mod color;

#[cfg(feature = "img")]
mod image;

#[cfg(feature = "ansi")]
pub mod term;

#[cfg(feature = "img")]
pub use image::Imgille;
