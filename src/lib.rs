#![warn(missing_docs)]
//! This crate is a rust lib for making [braille] art.
//!
//! You can use the basic canvas to paint something,
//! or you can use Turtle to paint things like in python.
//! And there are more useful things like colorful output, 3D object, life game and so on!
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
//! c.print();
//! ```
//!
//! draw a star
//! ```
//! use rsille::{extra::Turtle, Canvas};
//! let mut c = Canvas::new();
//! let mut t = Turtle::new();
//! for _ in 0..5 {
//!     t.forward(30.0);
//!     t.right(144.0);
//! }
//! c.paint(&t, 0.0, 15.0).unwrap();
//! c.print();
//! ```
//!
//! life game
//! ```no_run
//! use rsille::{extra::LifeGame, Animation};
//! let lg =  LifeGame::from_path("path/to/rle").unwrap();
//! let mut anime = Animation::new();
//! anime.push(lg, |lg| lg.update(), (0.0, 0.0));
//! anime.run();
//! ```
//!
//! Want more examples? check the [examples](https://github.com/nidhoggfgg/rsille/tree/main/examples)
//!
//! ## Extra
//!
//! Useful things can paint on canvas:
//! 1. [`Object3d`](extra/struct.Object3D.html) the 3d object
//! 2. [`Turtle`](extra/struct.Turtle.html) similar to the turtle in python
//! 3. [`Imagille`](extra/struct.Imgille.html) paint image to braille code
//! 4. [`Lifegame`](extra/struct.LifeGame.html) the life game in braille code
//!
//! It's inspired by [drawille], but it has more features and fast
//!
//! [braille]: http://www.alanwood.net/unicode/braille_patterns.html
//! [drawille]: https://github.com/asciimoo/drawille

mod anime;
mod braille;
mod canvas;
pub mod color;
mod defaults;
pub mod extra;
pub mod term;
mod utils;

pub use anime::Animation;
pub use canvas::Canvas;
pub use canvas::Paint;
