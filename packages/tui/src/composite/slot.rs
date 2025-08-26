use render::{area::Size, chunk::Chunk, Draw, DrawErr, Update};
use term::event::Event;

use crate::{
    attr::{Attr, SetAttr},
    Widget,
};

pub struct Slot {
    pub thing: Box<dyn Widget + Send + Sync>,
    pub updated: bool,
}

impl Draw for Slot {
    fn draw(&mut self, chunk: Chunk) -> Result<(), DrawErr> {
        self.thing.draw(chunk)
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
    fn get_attr(&self) -> &Attr {
        self.thing.get_attr()
    }

    fn set_attr(&mut self, attr: SetAttr) {
        self.thing.set_attr(attr);
    }

    fn size(&self) -> Size {
        self.thing.size()
    }
}
