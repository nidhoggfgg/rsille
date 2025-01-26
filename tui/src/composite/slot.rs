use term::event::Event;

use crate::{attr::Attr, style::Stylized, traits::Draw, DrawErr, DrawUpdate, Update};

pub struct Slot {
    pub attr: Attr,
    pub thing: Box<dyn DrawUpdate + Send + Sync>,
    pub updated: bool,
}

impl Draw for Slot {
    fn draw(&mut self) -> Result<Vec<Stylized>, DrawErr> {
        self.thing.draw()
    }

    fn size(&self) -> Option<(u16, u16)> {
        self.thing.size()
    }
}

impl Update for Slot {
    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        self.thing.on_events(events)
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        self.updated = self.thing.update()?;
        Ok(self.updated)
    }
}
