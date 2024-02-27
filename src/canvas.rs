// IMPORTANT: the algorithm is fixed, be very careful of changing the code
// there isn't a good way to debug

use std::io::Write;
use std::{cmp, collections::HashMap};

use crossterm::{cursor::MoveToNextLine, queue, style::Print};

use crate::braille;
use crate::utils::get_pos;
use crate::{
    braille::PixelOp,
    term::is_raw_mode,
    utils::{round, to_rsille_err, RsilleErr},
};

use crate::color::{Color, Colored};

/// Implement this for painting on [`Canvas`](struct.Canvas.html)
pub trait Paint: Send + 'static {
    /// Paint the object on the canvas
    fn paint<T>(&self, canvas: &mut Canvas, x: T, y: T) -> Result<(), RsilleErr>
    where
        T: Into<f64>;
}

// this is just for err: "Box<T> not impl Paint" xd
impl<T: Paint + ?Sized> Paint for Box<T> {
    fn paint<N>(&self, canvas: &mut Canvas, x: N, y: N) -> Result<(), RsilleErr>
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

#[derive(Debug, Clone)]
pub struct Canvas {
    minx: f64,                            // <= 0
    miny: f64,                            // <= 0
    width: i32,                           // >= 0
    height: i32,                          // >= 0
    pixels: HashMap<(i32, i32), Colored>, // (x, y) -> colored
}

impl Canvas {
    /// Make a new empty canvas
    ///
    /// The size of the canvas will auto increase
    pub fn new() -> Self {
        let pixels = HashMap::new();
        let (width, height) = (0, 0);
        let (minx, miny) = (0.0, 0.0);
        Self {
            minx,
            miny,
            width,
            height,
            pixels,
        }
    }

    /// Paint those [`Paint`]() object on the location (x, y)
    pub fn paint<T, N>(&mut self, target: &T, x: N, y: N) -> Result<(), RsilleErr>
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
        let is_raw = is_raw_mode();
        let mut stdout = std::io::stdout();
        self.print_on(&mut stdout, is_raw).unwrap();
    }

    /// Print the canvas to the buffer
    ///
    /// If you want to print the canvas to the terminal, use the [`print`](struct.Canvas.html#method.print)
    pub fn print_on<W>(&self, w: &mut W, is_raw: bool) -> Result<(), RsilleErr>
    where
        W: Write,
    {
        self.print_impl(w, is_raw).map_err(to_rsille_err)
    }

    fn print_impl<W>(&self, w: &mut W, is_raw: bool) -> std::io::Result<()>
    where
        W: Write,
    {
        let (start_col, start_row) = get_pos(self.minx, self.miny);
        for y in (start_row..self.height).rev() {
            for x in start_col..self.width {
                if let Some(pixel) = self.pixels.get(&(x, y)) {
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
        self.minx = 0.0;
        self.miny = 0.0;
        self.width = 0;
        self.height = 0;
        self.pixels = HashMap::new();
    }

    /// Set the size of the canvas
    ///
    /// This method can't fix the size of the canvas, it's just set the canvas size.
    /// When the size isn't enough, the canvas will auto increase.
    /// And the (width, height) isn't the size of the terminal, it's the size of the canvas!
    /// For example, an object `x` from -30 to 30, then it's 60 in width.
    /// On the terminal, it's 30 in width(because braille code), but you should set the width to 60 not 30.
    pub fn set_size<T>(&mut self, width: T, height: T)
    where
        T: Into<f64>,
    {
        // start_col, start_row < 0
        let (max_col, max_row) = get_pos(width.into(), height.into());
        let (start_col, start_row) = get_pos(self.minx, self.miny);
        if max_col > self.width - start_col {
            self.width = max_col + start_col;
        }
        if max_row > self.height - start_row {
            self.height = max_row + start_row;
        }
    }

    /// Set the min `x` of th canvas
    ///
    /// In most time, no need to call this, only when the animation is moved when running
    pub fn set_minx<T>(&mut self, minx: T)
    where
        T: Into<f64>,
    {
        let minx = minx.into();
        if minx < self.minx {
            self.minx = minx;
        }
    }

    /// Set the max `y` of the canvas
    ///
    /// In most time, no need to call this, only when the animation is moved when running
    pub fn set_maxy<T>(&mut self, maxy: T)
    where
        T: Into<f64> + Copy,
    {
        let maxy = maxy.into();
        let (_, max_row) = get_pos(0.0, maxy);
        if max_row > self.height {
            self.height = max_row;
        }
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
    pub fn set_colorful<T>(&mut self, x: T, y: T, color: Color)
    where
        T: Into<f64> + Copy,
    {
        self.set_at(x, y, Some(color));
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
    pub fn line_colorful<T>(&mut self, xy1: (T, T), xy2: (T, T), color: Color)
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

    fn set_at<T>(&mut self, x: T, y: T, color: Option<Color>)
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
                .set_foregound_color(color);
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
        if x < self.minx {
            self.minx = x;
        }
        if y < self.miny {
            self.miny = y;
        }
        let (col, row) = get_pos(x, y);
        if row >= self.height {
            self.height = row.abs() + 1;
        }
        if col >= self.width {
            self.width = col.abs() + 1;
        }
        (col, row)
    }
}
