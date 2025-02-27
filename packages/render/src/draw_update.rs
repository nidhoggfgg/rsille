pub trait Draw {
    fn draw(&mut self) -> Result<DrawChunk, crate::DrawErr>;
}

pub trait Update {
    fn on_events(&mut self, events: &[term::event::Event]) -> Result<(), crate::DrawErr>;
    fn update(&mut self) -> Result<bool, crate::DrawErr>;
}

// this trait is for making trait object
pub trait DrawUpdate: Draw + Update {}

impl<T: Draw + Update> DrawUpdate for T {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrawChunk(pub Vec<term::style::Stylized>, pub (u16, u16));
