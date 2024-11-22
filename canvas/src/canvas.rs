// IMPORTANT: the algorithm is fixed, be very careful of changing the code
// there isn't a good way to debug

use std::io::Write;
use std::{cmp, collections::HashMap};

use crossterm::style::Colors;
use crossterm::{cursor::MoveToNextLine, queue, style::Print};

use crate::braille::{self, PixelOp};
use crate::color::Colored;
use crate::utils::{get_pos, round};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PaintErr {}

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
        canvas.paint(self, x, y)
    }
}

/// The basic canvas
///
/// Paint anything on the canvas anywhere you want.
/// Don't worry about the (x, y), the size of canvas will auto increase,
/// and it support the negative number.
///
/// ## Example
///
/// Draw the `y = 1.5*sin(x)` and `y = cos(x)`
/// ```
/// use rsille::Canvas;
/// let mut c = Canvas::new();
/// for x in 0..1000 {
///     let x = x as f64;
///     c.set(x / 10.0, x.to_radians().sin() * 15.0);
///     c.set(x / 10.0, x.to_radians().cos() * 10.0);
/// }
/// c.print();
/// ```
///
/// ## NOTE
///
/// Take a look at the [`extra`](extra/index.html) module, there are some useful things can paint on the canvas
///

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Canvas {
    minx: i32,                            // <= 0
    miny: i32,                            // <= 0
    maxx: i32,                            // >= 0
    maxy: i32,                            // >= 0
    pixels: HashMap<(i32, i32), Colored>, // (col, row) -> colored
}

impl Canvas {
    /// Make a new empty canvas
    ///
    /// The size of the canvas will auto increase
    pub fn new() -> Self {
        let pixels = HashMap::new();
        let (width, height) = (0, 0);
        let (minx, miny) = (0, 0);
        Self {
            minx,
            miny,
            maxx: width,
            maxy: height,
            pixels,
        }
    }

    /// Paint those [`Paint`]() object on the location (x, y)
    pub fn paint<T, N>(&mut self, target: &T, x: N, y: N) -> Result<(), PaintErr>
    where
        T: Paint,
        N: Into<f64>,
    {
        let (x, y) = (x.into(), y.into());
        target.paint(self, x, y)?;
        Ok(())
    }

    /// Print the canvas to the terminal
    ///
    /// If you want to print the canvas to a buffer, use the [`print_on`](struct.Canvas.html#method.print_on)
    pub fn print(&self) {
        let is_raw = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
        let mut stdout = std::io::stdout();
        self.print_on(&mut stdout, is_raw).unwrap();
    }

    /// Print the canvas to the buffer
    ///
    /// If you want to print the canvas to the terminal, use the [`print`](struct.Canvas.html#method.print)
    fn print_on<W>(&self, w: &mut W, is_raw: bool) -> std::io::Result<()>
    where
        W: Write,
    {
        for row in (self.miny..self.maxy).rev() {
            for col in self.minx..self.maxx {
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

    /// Clear the canvas
    ///
    /// This method only clear those dots on the canvas, the size of the canvas will not change
    /// If you want to clear the size too, use the [`reset`](struct.Canvas.html#method.reset)
    pub fn clear(&mut self) {
        self.pixels = HashMap::new();
    }

    /// Reset the canvas to a new empty canvas
    pub fn reset(&mut self) {
        self.minx = 0;
        self.miny = 0;
        self.maxx = 0;
        self.maxy = 0;
        self.pixels = HashMap::new();
    }

    /// Draw a dot on (x, y)
    ///
    /// Just use the (x, y) in your object, the algorithm will find the right location
    pub fn set<T>(&mut self, x: T, y: T)
    where
        T: Into<f64> + Copy,
    {
        self.set_at(x, y, None);
    }

    /// Similar to [`set`](struct.Canvas.html#method.set)
    ///
    /// But it's support color
    pub fn set_colorful<T>(&mut self, x: T, y: T, colors: Colors)
    where
        T: Into<f64> + Copy,
    {
        self.set_at(x, y, Some(colors));
    }

    /// If the (x, y) is already set, then unset it
    ///
    /// If the (x, y) is unset, then set it
    pub fn toggle<T>(&mut self, x: T, y: T)
    where
        T: Into<f64> + Copy,
    {
        self.toggle_at(x, y);
    }

    /// Draw a line on the canvas
    /// * `xy1` - the start location
    /// * `xy2` - the end location
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

    /// Draw a line on the canvas with the color
    /// * `xy1` - the start location
    /// * `xy2` - the end location
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

    fn set_at<T>(&mut self, x: T, y: T, color: Option<Colors>)
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
        if let Some(color) = color {
            self.pixels
                .get_mut(&(col, row))
                .unwrap()
                .set_foregound_color(color.foreground.unwrap())
                .set_background_color(color.background.unwrap()); // TODO
        }
    }

    fn toggle_at<T>(&mut self, x: T, y: T)
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

    fn get_pos<T>(&mut self, x: T, y: T) -> (i32, i32)
    where
        T: Into<f64>,
    {
        let (x, y) = (x.into(), y.into());
        let (col, row) = get_pos(x, y);
        if row < self.minx {
            self.minx = row;
        }
        if row >= self.maxy {
            self.maxy = row.abs() + 1;
        }
        if col < self.minx {
            self.minx = col
        }
        if col >= self.maxx {
            self.maxx = col.abs() + 1;
        }
        (col, row)
    }
}
