use std::io;

use term::crossterm::{queue, style::Print};

use crate::{chunk::Chunk, Builder, DrawUpdate};

pub struct Render<W> {
    chunk: Chunk,
    thing: Box<dyn DrawUpdate + Send + Sync>,
    out: W,
    clear: bool,
}

impl<W> Render<W>
where
    W: std::io::Write,
{
    pub fn render(&mut self) -> io::Result<()> {
        self.thing.draw(&mut self.chunk)?;

        if self.clear {
            queue!(self.out, term::crossterm::terminal::Clear(term::crossterm::terminal::ClearType::All))?;
        }

        for line in self.chunk.lines() {
            let mut w = 0;
            for c in line {
                w += c.width() as u16;
                if w > self.chunk.size.width {
                    break;
                }

                c.queue(&mut self.out)?
            }
            queue!(self.out, Print("\n"))?
        }

        Ok(())
    }

    pub(crate) fn from_builder<T>(builder: &Builder, thing: T, writer: W) -> Self
    where
        T: DrawUpdate + Send + Sync + 'static,
    {
        Self {
            chunk: Chunk::empty((builder.size.width, builder.size.height)),
            thing: Box::new(thing),
            out: writer,
            clear: builder.clear,
        }
    }
}

// impl<W> Draw for Render<W> {
//     fn draw(&mut self, chunk: &mut Chunk) -> Result<(), crate::DrawErr> {
//         self.thing.draw(chunk)
//     }
// }

// impl<W> Update for Render<W> {
//     fn on_events(&mut self, events: &[term::event::Event]) -> Result<(), crate::DrawErr> {
//         self.thing.on_events(events)
//     }

//     fn update(&mut self) -> Result<bool, crate::DrawErr> {
//         self.thing.update()
//     }
// }
