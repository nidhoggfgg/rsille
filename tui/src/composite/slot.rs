use term::event::Event;

use crate::{attr::Attr, style::Stylized, traits::Draw, widgets::Widget, DrawErr, Update};

pub struct Slot {
    pub thing: Box<dyn Widget + Send + Sync>,
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

impl Widget for Slot {
    fn show(&mut self) -> Result<Vec<Stylized>, DrawErr> {
        self.thing.show()
    }

    fn get_attr(&self) -> &Attr {
        self.thing.get_attr()
    }

    fn set_attr(&mut self, attr: Attr) {
        self.thing.set_attr(attr);
    }
}
