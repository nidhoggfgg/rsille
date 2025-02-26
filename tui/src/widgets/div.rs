use term::event::Event;

use crate::{
    attr::Attr, style::Stylized, traits::Draw, widgets::Widget, DrawErr, DrawUpdate, Update,
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
    fn draw(&mut self) -> Result<Vec<Stylized>, DrawErr> {
        self.thing.draw()
    }

    fn size(&self) -> Option<(u16, u16)> {
        self.thing.size()
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
    fn show(&mut self) -> Result<Vec<Stylized>, DrawErr> {
        self.draw()
    }

    fn get_attr(&self) -> &Attr {
        &self.attr
    }

    fn set_attr(&mut self, attr: Attr) {
        self.attr = attr
    }
}
