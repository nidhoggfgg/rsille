use std::borrow::Cow;

use render::{chunk::Chunk, style::StylizedLine, Draw, DrawErr, Update};
use term::event::Event;

use crate::{attr::Attr, Widget};

pub struct Text<'a> {
    text: Vec<StylizedLine<'a>>,
    attr: Attr,
    updated: bool,
}

impl<'a> Text<'a> {
    pub fn new<T>(text: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        let lines = match text.into() {
            Cow::Borrowed(s) => s.lines().map(StylizedLine::from).collect(),
            Cow::Owned(s) => s
                .lines()
                .map(|x| StylizedLine::from(x.to_owned()))
                .collect(),
        };
        Self {
            text: lines,
            updated: true,
            attr: Default::default(),
        }
    }

    pub fn replace<T>(&mut self, text: T)
    where
        T: Into<Cow<'a, str>>,
    {
        let lines = match text.into() {
            Cow::Borrowed(s) => s.lines().map(StylizedLine::from).collect(),
            Cow::Owned(s) => s
                .lines()
                .map(|x| StylizedLine::from(x.to_owned()))
                .collect(),
        };

        self.text = lines;
        self.updated = true;
    }
}

impl Draw for Text<'_> {
    fn draw(&mut self, mut chunk: Chunk) -> Result<(), DrawErr> {
        for (y, line) in self.text.iter().enumerate() {
            for (x, c) in line.content.iter().flat_map(|x| x.into_iter()).enumerate() {
                if let Some(t) = chunk.get_mut(x as u16, y as u16) {
                    t.set_char(c.c.unwrap());
                }
            }
        }
        Ok(())
    }
}

impl Update for Text<'_> {
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

impl Widget for Text<'_> {
    fn get_attr(&self) -> &crate::attr::Attr {
        &self.attr
    }

    fn set_attr(&mut self, attr: crate::attr::SetAttr) {
        self.attr.set(attr);
    }
}
