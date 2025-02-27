pub trait Draw {
    fn draw(&mut self) -> Result<Vec<term::style::Stylized>, crate::DrawErr>;
    fn size(&self) -> Option<(u16, u16)>;
}

pub trait Update {
    fn on_events(&mut self, events: &[term::event::Event]) -> Result<(), crate::DrawErr>;
    fn update(&mut self) -> Result<bool, crate::DrawErr>;
}

// this trait is for making trait object
pub trait DrawUpdate: Draw + Update {}

impl<T: Draw + Update> DrawUpdate for T {}
