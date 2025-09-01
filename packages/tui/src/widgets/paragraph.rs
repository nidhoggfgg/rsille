use std::borrow::Cow;

use render::{area::Size, chunk::Chunk, style::StylizedLine, Draw, DrawErr};

#[derive(Debug, Clone)]
pub struct Paragraph {
    pub text: Vec<StylizedLine<'static>>,
}

impl Paragraph {
    pub fn new<T>(text: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        let lines: Vec<StylizedLine<'static>> = match text.into() {
            Cow::Borrowed(s) => s.lines().map(StylizedLine::from).collect(),
            Cow::Owned(s) => s
                .lines()
                .map(|x| StylizedLine::from(x.to_owned()))
                .collect(),
        };
        Self { text: lines }
    }
}

impl Draw for Paragraph {
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        let mut width = 0;
        let mut height = 0;
        for (y, line) in self.text.iter().enumerate() {
            let mut real_x = 0;
            for c in line.content.iter().flat_map(|x| x.into_iter()) {
                if let Ok(l) = chunk.set(real_x, y as u16, c) {
                    real_x += l as u16;
                } else {
                    break;
                }
            }
            if real_x > width {
                width = real_x;
            }
            height = y + 1;
        }
        Ok((width, height as u16).into())
    }
}
