use crate::style::Stylized;

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
pub struct DrawChunk(pub Vec<Stylized>, pub usize);

impl DrawChunk {
    pub fn padding(&mut self) {
        let tmp = self.0.len() % self.1;
        if tmp != 0 {
            let pad_num = self.1 - tmp;
            self.0.extend(vec![Stylized::nop(); pad_num]);
        }
    }
}
