// IMPORTANT: the algorithm is fixed, be very careful of changing the code
// there isn't a good way to debug

use std::io::Write;
use std::{cmp, collections::HashMap};

use crossterm::{cursor::MoveToNextLine, queue, style::Print};

use crate::braille;
use crate::utils::Toi32;
use crate::{
    braille::PixelOp,
    term::is_raw_mode,
    utils::{round, to_rsille_err, RsilleErr},
};

use crate::color::{Color, Colored};

/// implement this for painting on [`Canvas`](struct.Canvas.html)
pub trait Paint: Send + 'static {
    /// paint the object on the canvas
    fn paint(&self, canvas: &mut Canvas, x: f64, y: f64) -> Result<(), RsilleErr>;
}

// this is just for err: "Box<T> not impl Paint" xd
impl<T: Paint + ?Sized> Paint for Box<T> {
    fn paint(&self, canvas: &mut Canvas, x: f64, y: f64) -> Result<(), RsilleErr> {
        canvas.paint(self, x, y)
    }
}

/// The basic canvas
///
/// ## Example
///
/// draw the `y = 1.5*sin(x)` and `y = cos(x)`
/// ```
/// use rsille::Canvas;
/// let mut c = Canvas::new();
/// for x in 0..1000 {
///     let x = x as f64;
///     c.set(x / 10.0, 15.0 + x.to_radians().sin() * 15.0);
///     c.set(x / 10.0, 15.0 + x.to_radians().cos() * 10.0);
/// }
/// println!("{}", c.render());
/// ```
///
/// ## NOTE
///
/// take a look at the [`extra`](extra/index.html) module, it has some useful things can paint on the canvas
///
/// don't worry about the `x` and `y` in the canvas, it's not the location in the terminal.
/// use the `x` and `y` of your own algorithms, but don't let it less than `0`

#[derive(Debug, Clone)]
pub struct Canvas {
    minx: f64,   // <= 0
    miny: f64,   // <= 0
    width: i32,  // >= 0
    height: i32, // >= 0
    pixels: HashMap<(i32, i32), Colored>,
}

impl Canvas {
    /// make a new empty canvas
    ///
    /// the size of the canvas will auto increase
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

    /// paint those Paintable object on the location (x, y)
    pub fn paint(&mut self, target: &dyn Paint, x: f64, y: f64) -> Result<(), RsilleErr> {
        if x < 0.0 || y < 0.0 {
            return Err(RsilleErr::new(format!(
                "can't paint on postion {:#?}!",
                (x, y)
            )));
        }
        target.paint(self, x, y)?;
        Ok(())
    }

    /// print the canvas to the terminal
    ///
    /// if you want to print the canvas to a buffer, use the [`print_on`](struct.Canvas.html#method.print_on)
    pub fn print(&self) -> Result<(), RsilleErr> {
        let is_raw = is_raw_mode()?;
        let mut stdout = std::io::stdout();
        self.print_on(&mut stdout, is_raw)?;
        Ok(())
    }

    /// print the canvas to the buffer
    ///
    /// if you want to print the canvas to the terminal, use the [`print`](struct.Canvas.html#method.print)
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
        let (start_row, start_col) = self.get_pos_impl(self.minx, self.miny);
        for y in (start_row..self.height).rev() {
            for x in start_col..self.width {
                if let Some(pixel) = self.pixels.get(&(y, x)) {
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

    /// return the string prepare to print
    pub fn render(&self) -> String {
        self.get_lines().join("\n")
    }

    /// return the lines of the canvas
    pub fn get_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        let (px, py) = (self.minx.round() as i32, self.miny.round() as i32);
        for y in py..self.height {
            let mut buffer = Vec::new();
            for x in px..self.width {
                if let Some(pixel) = self.pixels.get(&(y, x)) {
                    pixel
                        .queue(&mut buffer)
                        .expect("Internal error: please report this issue!");
                } else {
                    queue!(buffer, Print(braille::SPACE)).unwrap();
                }
            }
            buffer.flush().unwrap();
            out.push(String::from_utf8(buffer).unwrap());
        }
        out
    }

    /// clear the canvas
    pub fn clear(&mut self) {
        self.pixels = HashMap::new();
    }

    /// reset the canvas to a new empty canvas
    pub fn reset(&mut self) {
        self.minx = 0.0;
        self.miny = 0.0;
        self.width = 0;
        self.height = 0;
        self.pixels = HashMap::new();
    }

    /// set the size of the canvas
    ///
    /// This method can't fix the size of the canvas, it's just set the canvas size,
    /// when the size isn't enough, the canvas will auto increase.
    /// And the (width, height) isn't the size of the terminal, it's the size of the canvas!
    /// For example, an object `x` from -30 to 30, then it's 60 in width,
    /// but on the terminal, it's 30 in width(because braille code), you should set the width to 60.
    pub fn set_size<T>(&mut self, width: T, height: T)
    where
        T: Toi32,
    {
        // srow < 0, scol < 0
        let (height, width) = self.get_pos_impl(width.to_i32() as f64, height.to_i32() as f64);
        let (srow, scol) = self.get_pos_impl(self.minx, self.miny);
        if width > self.width - scol {
            self.width = width + scol;
        }
        if height > self.height - srow {
            self.height = height + srow;
        }
    }

    /// draw a dot on (x, y)
    ///
    /// just use the (x, y) in your object, the algorithm will find the right location
    pub fn set(&mut self, x: f64, y: f64) {
        self.set_at(x, y, None);
    }

    /// similar to [`set`](struct.Canvas.html#method.set)
    /// but it's support color
    pub fn set_colorful(&mut self, x: f64, y: f64, color: Color) {
        self.set_at(x, y, Some(color));
    }

    /// if the (x, y) is already set, then unset it
    ///
    /// if the (x, y) is unset, then set it
    pub fn toggle(&mut self, x: f64, y: f64) {
        self.toggle_at(x, y);
    }

    /// fill ⣿ at the (x, y)
    ///
    /// the (x, y) is the location on canvas, it's hard to use it rightly
    ///
    /// so don't use it unless you know what you are doing!
    pub fn set_fill(&mut self, x: f64, y: f64) {
        self.fill_at(x, y, None);
    }

    /// similar to [`set_fill`](struct.Canvas.html#method.set_fill), but support color
    ///
    /// don't use it unless you know what you are doing!
    pub fn set_fill_colorful(&mut self, x: f64, y: f64, color: Color) {
        self.fill_at(x, y, Some(color));
    }

    /// draw a line on the canvas
    /// * `xy1` - the start location
    /// * `xy2` = the end location
    pub fn line(&mut self, xy1: (f64, f64), xy2: (f64, f64)) {
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

    /// draw a line on the canvas with the color
    /// * `xy1` - the start location
    /// * `xy2` - the end location
    pub fn line_colorful(&mut self, xy1: (f64, f64), xy2: (f64, f64), color: Color) {
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

    fn set_at(&mut self, x: f64, y: f64, color: Option<Color>) {
        let (row, col) = self.get_pos(x, y);
        if let Some(pixel) = self.pixels.get_mut(&(row, col)) {
            pixel.set(x, y);
        } else {
            self.pixels.insert((row, col), Colored::new());
            self.pixels.get_mut(&(row, col)).unwrap().set(x, y);
        }
        if let Some(color) = color {
            self.pixels
                .get_mut(&(row, col))
                .unwrap()
                .set_foregound_color(color);
        }
    }

    fn toggle_at(&mut self, x: f64, y: f64) {
        let (row, col) = self.get_pos(x, y);
        if let Some(pixel) = self.pixels.get_mut(&(row, col)) {
            pixel.toggle(x, y);
        } else {
            self.pixels.insert((row, col), Colored::new());
            self.pixels.get_mut(&(row, col)).unwrap().toggle(x, y);
        }
    }

    // fn set_foreground_at(&mut self, x: f64, y: f64) {
    //     let (row, col) = get_pos(x, y);
    //     if let Some(pixel) = self.pixels.get(&(row, col)) {
    //         pixel.set(x, y);
    //     } else {
    //         self.pixels.insert((row, col), Colored::new());
    //         self.pixels.get(&(row, col)).unwrap().set(x, y);
    //     }
    // }

    fn fill_at(&mut self, x: f64, y: f64, color: Option<Color>) {
        let (row, col) = (x.round() as i32, y.round() as i32); // not get_pos
        if let Some(pixel) = self.pixels.get_mut(&(row, col)) {
            pixel.fill();
        } else {
            self.pixels.insert((row, col), Colored::new());
            self.pixels.get_mut(&(row, col)).unwrap().fill();
        }
        if let Some(color) = color {
            self.pixels
                .get_mut(&(row, col))
                .unwrap()
                .set_foregound_color(color);
        }
    }

    //      ^
    //      |
    //      +--+
    //      |  |
    //      |  |
    //      |  |
    //      |  |
    // -----+--+-->
    //      0
    fn get_pos(&mut self, x: f64, y: f64) -> (i32, i32) {
        if x < self.minx {
            self.minx = x;
        }
        if y < self.miny {
            self.miny = y;
        }
        let (row, col) = self.get_pos_impl(x, y);
        if row.abs() >= self.height {
            self.height = row.abs() + 1;
        }
        if col.abs() >= self.width {
            self.width = col.abs() + 1;
        }
        (row, col)
    }

    fn get_pos_impl(&self, x: f64, y: f64) -> (i32, i32) {
        let (x, y) = (round(x), round(y));
        let row = if y < 0 { (y + 1) / 4 - 1 } else { y / 4 };
        let col = if x < 0 { (x - 1) / 2 } else { x / 2 };
        (row, col)
    }
}

// fn get_pos(x: f64, y: f64) -> (usize, usize) {
//     (y.round() as usize / 4, x.round() as usize / 2)
// }

// fn make_vec(len: usize) -> Vec<Colored> {
//     vec![Colored::new(); len]
// }
