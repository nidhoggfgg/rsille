//! Some useful things can paint on the canvas

#[cfg(feature = "img")]
mod imgille;
mod lifegame;
pub mod math;
mod object3d;
mod turtle;

#[cfg(feature = "img")]
pub use imgille::Imgille;
pub use lifegame::LifeGame;
pub use object3d::Object3D;
pub use turtle::Turtle;
