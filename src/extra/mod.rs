//! Some useful things can paint on the canvas

mod lifegame;
pub mod math;
mod object3d;
pub mod particles;
mod turtle;

pub use lifegame::LifeGame;
pub use object3d::Object3D;
pub use turtle::Turtle;

#[cfg(feature = "image")]
mod imgille;
#[cfg(feature = "image")]
pub use imgille::Imgille;
