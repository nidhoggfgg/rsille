use std::io::Write;
use std::{cmp, collections::HashMap};

use crossterm::style::Colors;
use crossterm::{cursor::MoveToNextLine, queue, style::Print};

use crate::braille::{self, PixelOp};
use crate::color::Colored;
use crate::utils::{get_pos, round};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaintErr;

/// Implement this for painting on [`Canvas`](struct.Canvas.html)
pub trait Paint: Send + 'static {
    /// Paint the object on the canvas
    fn paint<T>(&self, canvas: &mut Canvas, x: T, y: T) -> Result<(), PaintErr>
    where
        T: Into<f64>;
}

// this is just for err: "Box<T> not impl Paint" xd
impl<T: Paint + ?Sized> Paint for Box<T> {
    fn paint<N>(&self, canvas: &mut Canvas, x: N, y: N) -> Result<(), PaintErr>
    where
        N: Into<f64>,
    {
        // in canvas.paint, it will call Paint.paint
        // but this won't fall into
        // canvas.paint -> Paint.paint -> canvas.paint
        //                 +-- not this function, it's that one upon there
        canvas.paint(self, x, y)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Canvas {
    minx: i32,                            // <= 0
    miny: i32,                            // <= 0
    maxx: i32,                            // >= 0
    maxy: i32,                            // >= 0
    pixels: HashMap<(i32, i32), Colored>, // (col, row) -> colored
    unbound: bool,
}

impl Canvas {
    pub fn new() -> Self {
        let pixels = HashMap::new();
        let (maxx, maxy) = (0, 0);
        let (minx, miny) = (0, 0);
        Self {
            minx,
            miny,
            maxx,
            maxy,
            pixels,
            unbound: true,
        }
    }

    pub fn paint<T, N>(&mut self, target: &T, x: N, y: N) -> Result<(), PaintErr>
    where
        T: Paint,
        N: Into<f64>,
    {
        let (x, y) = (x.into(), y.into());
        target.paint(self, x, y)?;
        Ok(())
    }

    pub fn print(&self) {
        let is_raw = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
        let mut stdout = std::io::stdout();
        self.print_on(&mut stdout, is_raw).unwrap();
    }

    pub fn print_on<W>(&self, w: &mut W, is_raw: bool) -> std::io::Result<()>
    where
        W: Write,
    {
        for row in (self.miny..=self.maxy).rev() {
            for col in self.minx..=self.maxx {
                if let Some(pixel) = self.pixels.get(&(col, row)) {
                    pixel.queue(w)?;
                } else {
                    queue!(w, Print(braille::SPACE))?;
                }
            }
            if is_raw {
                queue!(w, MoveToNextLine(1))?;
            } else {
                queue!(w, Print("\n"))?;
            }
        }
        w.flush()?;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.pixels = HashMap::new();
    }

    pub fn reset(&mut self) {
        self.minx = 0;
        self.miny = 0;
        self.maxx = 0;
        self.maxy = 0;
        self.clear();
    }

    pub fn set<T>(&mut self, x: T, y: T)
    where
        T: Into<f64> + Copy,
    {
        let (col, row) = self.get_pos(x, y);
        if let Some(pixel) = self.pixels.get_mut(&(col, row)) {
            pixel.set(x, y);
        } else {
            self.pixels.insert((col, row), Colored::new());
            self.pixels.get_mut(&(col, row)).unwrap().set(x, y);
        }
    }

    pub fn set_colorful<T>(&mut self, x: T, y: T, colors: Colors)
    where
        T: Into<f64> + Copy,
    {
        self.set(x, y);
        let (col, row) = self.get_pos(x, y);
        self.pixels
            .get_mut(&(col, row))
            .unwrap()
            .set_foregound_color(colors.foreground.unwrap())
            .set_background_color(colors.background.unwrap());
    }

    pub fn toggle<T>(&mut self, x: T, y: T)
    where
        T: Into<f64> + Copy,
    {
        let (col, row) = self.get_pos(x, y);
        if let Some(pixel) = self.pixels.get_mut(&(col, row)) {
            pixel.toggle(x, y);
        } else {
            self.pixels.insert((col, row), Colored::new());
            self.pixels.get_mut(&(col, row)).unwrap().toggle(x, y);
        }
    }

    pub fn line<T>(&mut self, xy1: (T, T), xy2: (T, T))
    where
        T: Into<f64>,
    {
        let (x1, y1) = (round(xy1.0), round(xy1.1));
        let (x2, y2) = (round(xy2.0), round(xy2.1));
        let d = |v1, v2| {
            if v1 <= v2 {
                (v2 - v1, 1.0)
            } else {
                (v1 - v2, -1.0)
            }
        };

        let (xdiff, xdir) = d(x1, x2);
        let (ydiff, ydif) = d(y1, y2);
        let r = cmp::max(xdiff, ydiff);

        for i in 0..=r {
            let r = r as f64;
            let i = i as f64;
            let (xd, yd) = (xdiff as f64, ydiff as f64);
            let x = x1 as f64 + i * xd / r * xdir;
            let y = y1 as f64 + i * yd / r * ydif;
            self.set(x, y);
        }
    }

    pub fn line_colorful<T>(&mut self, xy1: (T, T), xy2: (T, T), color: Colors)
    where
        T: Into<f64> + Copy,
    {
        let (x1, y1) = (round(xy1.0), round(xy1.1));
        let (x2, y2) = (round(xy2.0), round(xy2.1));
        let d = |v1, v2| {
            if v1 <= v2 {
                (v2 - v1, 1.0)
            } else {
                (v1 - v2, -1.0)
            }
        };

        let (xdiff, xdir) = d(x1, x2);
        let (ydiff, ydif) = d(y1, y2);
        let r = cmp::max(xdiff, ydiff);

        for i in 0..=r {
            let r = r as f64;
            let i = i as f64;
            let (xd, yd) = (xdiff as f64, ydiff as f64);
            let x = x1 as f64 + i * xd / r * xdir;
            let y = y1 as f64 + i * yd / r * ydif;
            self.set_colorful(x, y, color);
        }
    }

    fn get_pos<T>(&mut self, x: T, y: T) -> (i32, i32)
    where
        T: Into<f64>,
    {
        // let (x, y) = (x.into(), y.into());
        let (col, row) = get_pos(x, y);
        if row < self.minx {
            self.minx = row;
        }
        if row >= self.maxy {
            self.maxy = row.abs();
        }
        if col < self.minx {
            self.minx = col
        }
        if col >= self.maxx {
            self.maxx = col.abs();
        }
        (col, row)
    }
}
