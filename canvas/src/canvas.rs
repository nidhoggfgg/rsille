use std::io::Write;
use std::{cmp, collections::HashMap};

use crossterm::{cursor::MoveToNextLine, queue, style::Print};

use crate::bound::Bound;
use crate::braille::{self, PixelOp};

use crate::utils::{get_pos, round};

#[cfg(feature = "no_color")]
use crate::braille::Pixel;

#[cfg(not(feature = "no_color"))]
use crate::color::Colored;
#[cfg(not(feature = "no_color"))]
use crossterm::style::Colors;

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

#[repr(C)]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Canvas {
    bound: Bound,
    #[cfg(not(feature = "no_color"))]
    pixels: HashMap<(i32, i32), Colored>,
    #[cfg(feature = "no_color")]
    pixels: HashMap<(i32, i32), Pixel>,
    unbound: bool,
}

impl Canvas {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        let pixels = HashMap::new();
        Self {
            bound: Bound::new(),
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
        let ((minx, maxx), (miny, maxy)) = self.bound.get_bound();
        for row in (miny..=maxy).rev() {
            for col in minx..=maxx {
                if let Some(pixel) = self.pixels.get(&(col, row)) {
                    pixel.queue(w)?;
                } else {
                    queue!(w, Print(braille::BRAILLE_SPACE))?;
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
        self.bound = Bound::new();
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
            #[cfg(not(feature = "no_color"))]
            self.pixels.insert((col, row), Colored::new());
            #[cfg(feature = "no_color")]
            self.pixels.insert((col, row), Pixel::new());
            self.pixels.get_mut(&(col, row)).unwrap().set(x, y);
        }
    }

    #[cfg(not(feature = "no_color"))]
    pub fn set_colorful<T>(&mut self, x: T, y: T, colors: Colors)
    where
        T: Into<f64> + Copy,
    {
        self.set(x, y);
        let (col, row) = self.get_pos(x, y);
        if let Some(pixel) = self.pixels.get_mut(&(col, row)) {
            if let Some(foreground) = colors.foreground {
                pixel.set_foregound_color(foreground);
            }
            if let Some(background) = colors.background {
                pixel.set_background_color(background);
            }
        } else {
            unreachable!("pixel not found")
        }
    }

    pub fn toggle<T>(&mut self, x: T, y: T)
    where
        T: Into<f64> + Copy,
    {
        let (col, row) = self.get_pos(x, y);
        if let Some(pixel) = self.pixels.get_mut(&(col, row)) {
            pixel.toggle(x, y);
        } else {
            #[cfg(not(feature = "no_color"))]
            self.pixels.insert((col, row), Colored::new());
            #[cfg(feature = "no_color")]
            self.pixels.insert((col, row), Pixel::new());
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

    #[cfg(not(feature = "no_color"))]
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

    pub fn size(&self) -> (i32, i32) {
        let ((minx, maxx), (miny, maxy)) = self.bound.get_bound();
        return (maxx - minx + 1, maxy - miny + 1);
    }

    fn get_pos<T>(&mut self, x: T, y: T) -> (i32, i32)
    where
        T: Into<f64>,
    {
        // let (x, y) = (x.into(), y.into());
        let (col, row) = get_pos(x, y);
        self.bound.update(col, row);
        (col, row)
    }
}
