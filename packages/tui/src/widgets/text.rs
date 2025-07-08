use render::{style::Stylized, Draw, DrawChunk, DrawErr, Update};
use term::event::Event;
use unicode_width::UnicodeWidthChar;

use crate::{
    attr::{Attr, ElementSize},
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
    fn draw(&mut self) -> Result<DrawChunk, DrawErr> {
        let max_height = match self.attr.height {
            ElementSize::Auto => usize::MAX,
            ElementSize::Fixed(height) => height.into(),
        };
        let max_width = match self.attr.width {
            ElementSize::Auto => usize::MAX,
            ElementSize::Fixed(width) => width.into(),
        };

        let mut result = Vec::new();
        for (h, line) in self.text.iter().enumerate() {
            if h >= max_height {
                break;
            }
            let mut line_chars = Vec::new();
            let mut w = 0;
            for c in line.chars() {
                if w >= max_width {
                    break;
                }
                line_chars.push(Stylized::new(c, None, None));
                w += c.width_cjk().unwrap_or(0);
            }
            result.push(line_chars);
        }
        Ok(DrawChunk::Chunk(result))
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
