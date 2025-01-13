use term::crossterm::event::Event;

use crate::{style::Stylized, DrawErr};

pub trait Draw {
    fn draw(&self) -> Result<Vec<Stylized>, DrawErr>;
    fn size(&self) -> Option<(u16, u16)>;
}

pub trait Update: Send {
    fn update(&mut self, events: &[Event]) -> Result<bool, DrawErr>;
}

// this trait is for making trait object
pub trait DrawUpdate: Draw + Update {}

impl<T: Draw + Update> DrawUpdate for T {}
