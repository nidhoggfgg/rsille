use crate::{DrawErr, chunk::Chunk};

pub trait Draw {
    fn draw(&mut self, chunk: &mut Chunk) -> Result<(), DrawErr>;
}

pub trait Update {
    fn on_events(&mut self, events: &[term::event::Event]) -> Result<(), DrawErr>;
    fn update(&mut self) -> Result<bool, DrawErr>;
}

// this trait is for making trait object
pub trait DrawUpdate: Draw + Update {}

impl<T: Draw + Update> DrawUpdate for T {}
