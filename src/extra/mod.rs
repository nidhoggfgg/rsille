//! Some useful things can paint on the canvas

pub mod math;
pub mod object3d;
pub mod particles;

mod turtle;
mod lifegame;

pub use lifegame::LifeGame;
pub use turtle::Turtle;
pub use object3d::Object3D;

#[cfg(feature = "image")]
mod imgille;
#[cfg(feature = "image")]
pub use imgille::Imgille;

#[cfg(feature = "colorgrad")]
pub mod rainbow;
