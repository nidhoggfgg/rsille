use render::{Draw, DrawErr, DrawUpdate, Update};
use term::event::Event;

use crate::{
    attr::{Attr, SetAttr},
    Widget,
};

pub struct Div {
    pub attr: Attr,
    thing: Box<dyn DrawUpdate + Send + Sync>,
}

impl Div {
    pub fn new<T>(thing: T) -> Self
    where
        T: DrawUpdate + Send + Sync + 'static,
    {
        Self {
            attr: Default::default(),
            thing: Box::new(thing),
        }
    }
}

impl Draw for Div {
    fn draw(&mut self) -> Result<render::DrawChunk, render::DrawErr> {
        self.thing.draw()
    }
}

impl Update for Div {
    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        self.thing.on_events(events)
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        self.thing.update()
    }
}

impl Widget for Div {
    fn get_attr(&self) -> &Attr {
        &self.attr
    }

    fn set_attr(&mut self, attr: SetAttr) {
        self.attr.set(attr);
    }
}
