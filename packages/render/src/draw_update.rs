use crate::{DrawErr, area::Size, chunk::Chunk};

pub trait Draw {
    fn draw(&mut self, chunk: Chunk) -> Result<Size, DrawErr>;
}

pub trait Update {
    fn on_events(&mut self, events: &[term::event::Event]) -> Result<(), DrawErr>;
    fn update(&mut self) -> Result<bool, DrawErr>;
}

// this trait is for making trait object
pub trait DrawUpdate: Draw + Update {}

impl<T: Draw + Update> DrawUpdate for T {}
