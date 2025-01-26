use term::event::Event;

use crate::{style::Stylized, DrawErr};

pub trait Draw {
    fn draw(&mut self) -> Result<Vec<Stylized>, DrawErr>;
    fn size(&self) -> Option<(u16, u16)>;
}

pub trait Update {
    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr>;
    fn update(&mut self) -> Result<bool, DrawErr>;
}

// this trait is for making trait object
pub trait DrawUpdate: Draw + Update {}

impl<T: Draw + Update> DrawUpdate for T {}
