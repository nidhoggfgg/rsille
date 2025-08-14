use render::{chunk::Chunk, Draw, DrawErr, Update};
use term::event::Event;

use crate::{
    attr::Attr,
    Widget,
};

pub struct Text {
    origin: String,
    text: Vec<String>,
    attr: Attr,
    updated: bool,
}

impl Text {
    pub fn new(text: &str) -> Self {
        let origin = text.to_string();
        let text = text.split("\n").map(|x| x.into()).collect();
        Self {
            origin,
            text,
            updated: true,
            attr: Default::default(),
        }
    }

    pub fn replace(&mut self, text: &str) {
        self.origin = text.to_string();
        let text = text.split("\n").map(|x| x.into()).collect();
        self.text = text;
        self.updated = true;
    }

    pub fn get_text(&mut self) -> &str {
        &self.origin
    }
}

impl Draw for Text {
    fn draw(&mut self, chunk: &mut Chunk) -> Result<(), DrawErr> {
        for (y, line) in self.text.iter().enumerate() {
            for (x, c) in line.chars().enumerate() {
                if let Some(t) = chunk.get_mut(x as u16, y as u16) {
                    t.set_char(c);
                }
            }
        }
        Ok(())
    }
}

impl Update for Text {
    fn on_events(&mut self, _events: &[Event]) -> Result<(), DrawErr> {
        Ok(())
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        if self.updated {
            // next time call update will return false if the text doesn't changeed
            self.updated = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Widget for Text {
    fn get_attr(&self) -> &crate::attr::Attr {
        &self.attr
    }

    fn set_attr(&mut self, attr: crate::attr::SetAttr) {
        self.attr.set(attr);
    }
}
