use std::{cell::RefCell, io};

use term::crossterm::{cursor::MoveTo, queue, style::Print};

use crate::{Builder, Draw, DrawChunk, DrawUpdate, Update};

pub struct Render<W> {
    size: Size,
    home: (u16, u16),
    thing: RefCell<Box<dyn DrawUpdate + Send + Sync>>,
    out: RefCell<W>,
    clear: bool,
}

impl<W> Render<W>
where
    W: std::io::Write,
{
    pub fn render(&self) -> io::Result<()> {
        let data = self.thing.borrow_mut().draw()?;
        let (cur_col, mut cur_row) = self.home;
        if self.clear {
            term::clear()?
        }
        queue!(self.out.borrow_mut(), MoveTo(cur_col, cur_row))?;

        let (max_width, max_height) = match self.size {
            Size::Fixed(w, h) => (w, h),
            Size::FullScreen => {
                queue!(self.out.borrow_mut(), MoveTo(0, 0))?;
                term::terminal_size()?
            }
        };
        let (max_width, max_height) = (max_width as usize, max_height as usize);

        match data {
            DrawChunk::Chunk(data) => {
                for (height, line) in data.iter().enumerate() {
                    if height >= max_height {
                        break;
                    }

                    let mut now_width = 0;
                    for c in line {
                        let cw = c.width();
                        if now_width + cw > max_width {
                            break;
                        }
                        c.queue(self.out.borrow_mut().by_ref())?;
                        now_width += cw;
                    }
                    if now_width < max_width {
                        let n = max_width - now_width;
                        queue!(self.out.borrow_mut().by_ref(), Print(" ".repeat(n)))?
                    }
                    cur_row += 1;
                    queue!(self.out.borrow_mut(), MoveTo(cur_col, cur_row))?;
                }
            }
        }

        self.out.borrow_mut().flush()
    }
}

impl<W> Draw for Render<W> {
    fn draw(&mut self) -> Result<DrawChunk, crate::DrawErr> {
        self.thing.borrow_mut().draw()
    }
}

impl<W> Update for Render<W> {
    fn on_events(&mut self, events: &[term::event::Event]) -> Result<(), crate::DrawErr> {
        self.thing.borrow_mut().on_events(events)
    }

    fn update(&mut self) -> Result<bool, crate::DrawErr> {
        self.thing.borrow_mut().update()
    }
}

impl<W> Render<W>
where
    W: std::io::Write,
{
    pub(crate) fn from_builder<T>(builder: &Builder, thing: T, writer: W) -> Self
    where
        T: DrawUpdate + Send + Sync + 'static,
    {
        Self {
            home: builder.home,
            size: builder.size,
            thing: RefCell::new(Box::new(thing)),
            out: RefCell::new(writer),
            clear: builder.clear,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub(crate) enum Size {
    Fixed(u16, u16),
    FullScreen,
}
