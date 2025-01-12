use term::crossterm::event::Event;

use crate::{attr::Attr, style::Stylized, traits::Draw, DrawErr, DrawUpdate, Update};

pub struct Slot {
    pub attr: Attr,
    pub thing: Box<dyn DrawUpdate + Send + Sync>,
    pub updated: bool,
    pub enable_cache: bool,
}

impl Draw for Slot {
    fn draw(&self) -> Result<Vec<Stylized>, DrawErr> {
        self.thing.draw()
    }

    fn size(&self) -> Option<(u32, u32)> {
        self.thing.size()
    }
}

impl Update for Slot {
    fn update(&mut self, events: &[Event]) -> Result<bool, DrawErr> {
        self.updated = self.thing.update(events)?;
        Ok(self.updated)
    }
}
