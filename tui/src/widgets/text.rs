use unicode_width::UnicodeWidthChar;

use crate::{
    attr::{Attr, AttrDisplay},
    Draw, Stylized, Update,
};

use super::Widget;

pub struct Text {
    text: Vec<String>,
    attr: Attr,
    width: usize,
    height: usize,
    updated: bool,
}

impl Text {
    pub fn new(text: String) -> Self {
        let (text, (width, height)) = split(text);
        Self {
            text,
            width,
            height,
            updated: true,
            attr: Default::default(),
        }
    }

    pub fn replace(&mut self, text: String) {
        let (text, (width, height)) = split(text);
        self.text = text;
        self.width = width;
        self.height = height;
        self.updated = true;
    }
}

impl Draw for Text {
    fn draw(&mut self) -> Result<Vec<crate::Stylized>, crate::DrawErr> {
        let mut result = Vec::with_capacity(self.height * self.width);
        for l in &self.text {
            for c in l.chars() {
                result.push(Stylized::new(c, None, None));
            }
        }
        Ok(result)
    }

    fn size(&self) -> Option<(u16, u16)> {
        Some((self.width as u16, self.height as u16))
    }
}

impl Update for Text {
    fn on_events(&mut self, _events: &[term::event::Event]) -> Result<(), crate::DrawErr> {
        Ok(())
    }

    fn update(&mut self) -> Result<bool, crate::DrawErr> {
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

    fn show(&mut self) -> Result<Vec<Stylized>, crate::DrawErr> {
        if self.attr.display == AttrDisplay::Hidden {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let max_witdh = self.attr.width as usize;
        let max_height = self.attr.height as usize;
        for (height, line) in self.text.iter().enumerate() {
            if height >= max_height {
                break;
            }
            let mut width = 0;
            for c in line.chars() {
                let cw = c.width().unwrap_or(0);
                if width + cw > max_witdh {
                    break;
                }
                result.push(Stylized::new(c, None, None));
                width += cw;
            }
        }

        Ok(result)
    }
}

fn split(text: String) -> (Vec<String>, (usize, usize)) {
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
