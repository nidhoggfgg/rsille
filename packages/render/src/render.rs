use std::{
    cell::RefCell,
    io::{self, Stdout},
};

use term::crossterm::{
    cursor::{MoveTo, MoveToNextLine},
    queue,
    style::Print,
};

use crate::{DrawChunk, DrawErr, DrawUpdate};

pub struct Render<W> {
    size: Size,
    #[allow(unused)]
    home: (u16, u16), // TODO: support
    raw_mode: bool,
    thing: RefCell<Box<dyn DrawUpdate + Send + Sync>>,
    out: RefCell<W>,
}

impl Render<Stdout> {
    pub fn new<T>(thing: T) -> Self
    where
        T: DrawUpdate + Send + Sync + 'static,
    {
        Self {
            size: Size::FullScreen,
            home: (0, 0),
            raw_mode: false,
            thing: RefCell::new(Box::new(thing)),
            out: RefCell::new(std::io::stdout()),
        }
    }
}

impl<W> Render<W>
where
    W: std::io::Write,
{
    pub fn with<T>(w: W, thing: T) -> Self
    where
        T: DrawUpdate + Send + Sync + 'static,
    {
        Self {
            size: Size::FullScreen,
            home: (0, 0),
            raw_mode: false,
            thing: RefCell::new(Box::new(thing)),
            out: RefCell::new(w),
        }
    }

    pub fn render(&self) -> io::Result<()> {
        let DrawChunk(data, width) = self.thing.borrow_mut().draw()?;
        queue!(self.out.borrow_mut(), MoveTo(0, 0))?;
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
            if self.raw_mode {
                queue!(self.out.borrow_mut(), MoveToNextLine(1))?;
            } else {
                queue!(self.out.borrow_mut(), Print("\n"))?;
            }
        }

        self.out.borrow_mut().flush()
    }

    pub fn raw_mode(&mut self, enable: bool) -> &mut Self {
        self.raw_mode = enable;
        self
    }

    pub fn home(&mut self, home: (u16, u16)) -> &mut Self {
        self.home = home;
        self
    }

    pub fn size(&mut self, (col, row): (u16, u16)) -> &mut Self {
        self.size = Size::Fixed(col, row);
        self
    }

    pub fn fullscreen(&mut self) -> &mut Self {
        self.size = Size::FullScreen;
        self
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

// unimplemented
// this is useful, when want to use Render with other things
// like put a clock on the right top corner in shell
#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub(crate) enum Position {
    LeftTop,
    RightTop,
    LeftBottom,
    RightBottom,
    Center,
}
