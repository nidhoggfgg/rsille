use render::{style::Stylized, Draw, DrawChunk, DrawErr, Update};
use term::event::Event;

use crate::{attr::Attr, Widget};

pub struct Text {
    origin: String,
    text: Vec<String>,
    attr: Attr,
    width: usize,
    height: usize,
    updated: bool,
}

impl Text {
    pub fn new(text: &str) -> Self {
        let origin = text.to_string();
        let (text, (width, height)) = split(text);
        Self {
            origin,
            text,
            width,
            height,
            updated: true,
            attr: Attr {
                width: width as u16,
                height: height as u16,
                ..Default::default()
            },
        }
    }

    pub fn replace(&mut self, text: String) {
        let origin = text.clone();
        let (text, (width, height)) = split(&text);
        self.origin = origin;
        self.text = text;
        self.width = width;
        self.height = height;
        self.attr.width = width as u16;
        self.attr.height = height as u16;
        self.updated = true;
    }

    pub fn get_text(&mut self) -> &str {
        &self.origin
    }
}

impl Draw for Text {
    fn draw(&mut self) -> Result<DrawChunk, DrawErr> {
        let mut result = Vec::with_capacity(self.height * self.width);
        for l in &self.text {
            for c in l.chars() {
                result.push(Stylized::new(c, None, None));
            }
        }
        Ok(DrawChunk(result, self.width))
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

    fn set_attr(&mut self, attr: crate::attr::Attr) {
        self.attr = attr;
    }
}

fn split(text: &str) -> (Vec<String>, (usize, usize)) {
    let mut heigth = 0;
    let mut width = 0;
    let result = text
        .split("\n")
        .map(|x| {
            let w = x.chars().count();
            if w > width {
                width = w;
            }
            heigth += 1;
            x.into()
        })
        .collect();
    (result, (width, heigth))
}
