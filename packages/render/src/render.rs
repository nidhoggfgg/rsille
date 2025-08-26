use term::crossterm::{
    cursor::MoveTo,
    queue,
    style::Print,
    terminal::{Clear, ClearType},
};

use crate::{Builder, Draw, DrawErr, DrawUpdate, area::Position, buffer::Buffer, chunk::Chunk};

#[derive(Debug, Clone, Default)]
pub struct Render<W, T> {
    pub(crate) pos: Position,
    pub(crate) buffer: Buffer,
    pub(crate) thing: T,
    out: W,
    clear: bool,
    append_newline: bool,
}

impl<W, T> Render<W, T>
where
    W: std::io::Write,
    T: Draw + Sync + Send,
{
    pub fn render(&mut self) -> std::io::Result<()> {
        // the position in chunk should be (0, 0), the render already move to the target position
        let buffer_size = self.buffer.size();
        let chunk = Chunk::new(&mut self.buffer, buffer_size.into())?;
        self.thing.draw(chunk)?;

        if self.clear {
            queue!(self.out, Clear(ClearType::All))?;
        }

        queue!(self.out, MoveTo(self.pos.x, self.pos.y))?;

        for (y, line) in self
            .buffer
            .content()
            .chunks(buffer_size.width as usize)
            .enumerate()
        {
            if y >= buffer_size.height as usize {
                break;
            }

            for c in line {
                if c.is_occupied {
                    continue;
                }

                c.queue(&mut self.out)?
            }

            if y < buffer_size.height as usize - 1 {
                queue!(self.out, MoveTo(self.pos.x, self.pos.y + y as u16 + 1))?;
            }
        }

        if self.append_newline {
            queue!(self.out, Print("\n"))?;
        }

        Ok(())
    }

    pub(crate) fn from_builder(builder: &Builder, thing: T, writer: W) -> Self
    where
        T: Draw + Send + Sync + 'static,
        W: std::io::Write,
    {
        Self {
            pos: builder.pos,
            buffer: Buffer::new(builder.size),
            thing,
            out: writer,
            clear: builder.clear,
            append_newline: builder.append_newline,
        }
    }
}

// do not impl DrawUpdate for Render, it's not a drawable thing
impl<W, T> Render<W, T>
where
    W: std::io::Write,
    T: DrawUpdate + Send + Sync + 'static,
{
    pub fn update(&mut self) -> Result<bool, DrawErr> {
        self.thing.update()
    }

    pub fn on_events(&mut self, events: &[term::event::Event]) -> Result<(), DrawErr> {
        self.thing.on_events(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Draw, DrawErr, style::Stylized};

    struct Text {
        lines: Vec<String>,
    }

    impl Text {
        fn new(text: &str) -> Self {
            Self {
                lines: text.split("\n").map(|x| x.to_string()).collect(),
            }
        }
    }

    impl Draw for Text {
        fn draw(&mut self, mut chunk: Chunk) -> Result<(), DrawErr> {
            for (y, line) in self.lines.iter().enumerate() {
                let mut x = 0;
                for c in line.chars() {
                    if let Ok(w) = chunk.set(x as u16, y as u16, Stylized::raw(c)) {
                        x += w;
                    } else {
                        break;
                    }
                }
            }
            Ok(())
        }
    }

    #[test]
    fn test_render_simple() {
        let thing = Text::new("Hello, world!\nHello, world!");
        let mut buffer = Vec::new();
        let mut render = Builder::new()
            .size((10, 2))
            .clear(false)
            .build_render(thing, &mut buffer);
        render.render().unwrap();

        let result = String::from_utf8(buffer).unwrap();

        assert_eq!(result, "\u{1b}[1;1HHello, wor\u{1b}[2;1HHello, wor");
    }

    #[test]
    fn test_render_clear() {
        let thing = Text::new("Hello, world!");
        let mut buffer = Vec::new();
        let mut render = Builder::new()
            .size((20, 1))
            .clear(true)
            .build_render(thing, &mut buffer);
        render.render().unwrap();

        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(result, "\u{1b}[2J\u{1b}[1;1HHello, world!       ");
    }

    #[test]
    fn test_render_cut_off() {
        let thing = Text::new("你好啊啊啊");
        let mut buffer = Vec::new();
        let mut render = Builder::new()
            .size((5, 1))
            .clear(false)
            .build_render(thing, &mut buffer);
        render.render().unwrap();

        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(result, "\u{1b}[1;1H你好 ");
    }

    #[test]
    fn test_render_position() {
        let thing = Text::new("test line 1\ntest line 2");
        let mut buffer = Vec::new();
        let mut render = Builder::new()
            .size((10, 2))
            .clear(false)
            .pos((10, 10))
            .build_render(thing, &mut buffer);
        render.render().unwrap();

        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(result, "\u{1b}[11;11Htest line \u{1b}[12;11Htest line ");
    }
}
