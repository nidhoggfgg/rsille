use std::{cell::RefCell, io};

use term::crossterm::{cursor::MoveTo, queue, style::Print};

use crate::{Builder, Draw, DrawChunk, DrawErr, DrawUpdate, Update};

pub struct Render<W> {
    size: Size,
    #[allow(unused)]
    home: (u16, u16), // TODO: support
    thing: RefCell<Box<dyn DrawUpdate + Send + Sync>>,
    out: RefCell<W>,
    clear: bool,
}

// impl Render<Stdout> {
//     pub fn new<T>(thing: T) -> Self
//     where
//         T: DrawUpdate + Send + Sync + 'static,
//     {
//         Self {
//             size: Size::FullScreen,
//             home: (0, 0),
//             thing: RefCell::new(Box::new(thing)),
//             out: RefCell::new(std::io::stdout()),
//             clear: true,
//         }
//     }
// }

impl<W> Render<W>
where
    W: std::io::Write,
{
    // pub fn with<T>(w: W, thing: T) -> Self
    // where
    //     T: DrawUpdate + Send + Sync + 'static,
    // {
    //     Self {
    //         size: Size::FullScreen,
    //         home: (0, 0),
    //         thing: RefCell::new(Box::new(thing)),
    //         out: RefCell::new(w),
    //     }
    // }

    pub fn render(&self) -> io::Result<()> {
        let DrawChunk(data, width) = self.thing.borrow_mut().draw()?;
        let (cur_col, mut cur_row) = self.home;
        if self.clear {
            term::clear()?
        }
        queue!(self.out.borrow_mut(), MoveTo(cur_col, cur_row))?;

        if width == 0 {
            if data.is_empty() {
                return Ok(());
            } else {
                return Err(DrawErr.into());
            }
        }

        if data.len() % width != 0 {
            return Err(DrawErr.into());
        }

        let (max_width, max_height) = match self.size {
            Size::Fixed(w, h) => (w, h),
            Size::Auto => todo!(),
            Size::FullScreen => {
                queue!(self.out.borrow_mut(), MoveTo(0, 0))?;
                term::terminal_size()?
            }
        };
        let (max_width, max_height) = (max_width as usize, max_height as usize);

        for (height, chunk) in data.chunks(width).enumerate() {
            if height >= max_height {
                break;
            }

            let mut now_width = 0;
            for v in chunk {
                let cw = v.width();
                if now_width + cw > max_width {
                    break;
                }
                v.queue(self.out.borrow_mut().by_ref())?;
                now_width += cw;
            }
            if now_width < max_width {
                let n = max_width - now_width;
                queue!(self.out.borrow_mut().by_ref(), Print(" ".repeat(n)))?
            }
            cur_row += 1;
            queue!(self.out.borrow_mut(), MoveTo(cur_col, cur_row))?;
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

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub(crate) enum Size {
    Fixed(u16, u16),
    FullScreen,

    // unimplemented, bc panel have fixed size
    Auto,
}
