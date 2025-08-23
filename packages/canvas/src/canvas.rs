use std::cmp;
use std::collections::HashMap;
use std::io::Write;

use crate::bound::Bound;
use crate::braille::{Pixel, PixelOp};
use crate::tile::Tile;
use crate::utils::round;

use render::chunk::Chunk;
use render::{Draw, DrawErr};
use term::crossterm::cursor::MoveToNextLine;
use term::crossterm::queue;
use term::crossterm::style::Print;

use crate::color::Colored;
use term::crossterm::style::Colors;

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
        // Paint.paint -> canvas.paint -> Paint.paint -> canvas.paint ...
        //                                +-- not this function, it's that one upon there
        canvas.paint(self, x, y)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Canvas {
    bound: Bound,
    pixels: HashMap<Tile, Colored>,
}

impl Canvas {
    /// Creates a new empty canvas with default settings.
    ///
    /// The canvas starts with no pixels and an unbounded drawing area.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        let pixels = HashMap::new();
        Self {
            bound: Bound::new(),
            pixels,
        }
    }

    /// Paints a target object on the canvas at the specified coordinates.
    ///
    /// This method delegates the actual painting to the target object's `paint` method.
    pub fn paint<T, N>(&mut self, target: &T, x: N, y: N) -> Result<(), PaintErr>
    where
        T: Paint,
        N: Into<f64>,
    {
        let (x, y) = (x.into(), y.into());
        target.paint(self, x, y)?;
        Ok(())
    }

    /// Prints the canvas content to stdout.
    pub fn print(&self) {
        let is_raw = term::crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
        let mut stdout = std::io::stdout();
        self.print_on(&mut stdout, is_raw).unwrap();
    }

    /// Prints the canvas content to a specified writer.
    ///
    /// This method allows you to control where the canvas output goes and whether
    /// to use raw terminal mode formatting.
    pub fn print_on<W>(&self, w: &mut W, is_raw: bool) -> std::io::Result<()>
    where
        W: Write,
    {
        let ((minx, maxx), (miny, maxy)) = self.bound.get_bound();
        for row in (miny..=maxy).rev() {
            for col in minx..=maxx {
                if let Some(pixel) = self.pixels.get(&Tile::from(col, row)) {
                    pixel.queue(w)?;
                } else {
                    queue!(w, Print(Pixel::space()))?;
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

    /// Clears all pixels from the canvas.
    ///
    /// This removes all drawn content but preserves the canvas boundary settings.
    /// The canvas remains the same size but becomes empty.
    pub fn clear(&mut self) {
        self.pixels = HashMap::new();
    }

    /// Resets the canvas to its initial state.
    ///
    /// This clears all pixels and resets the boundary to default (unbounded).
    /// The canvas will be completely empty and ready for new drawing.
    pub fn reset(&mut self) {
        self.bound = Bound::new();
        self.clear();
    }

    /// Sets a pixel at the specified coordinates.
    pub fn set<T>(&mut self, x: T, y: T) -> &mut Self
    where
        T: Into<f64> + Copy,
    {
        let tile = self.get_tile(x, y);
        let (x, y) = (round(x), round(y));
        if let Some(pixel) = self.pixels.get_mut(&tile) {
            pixel.set(x, y);
        } else {
            let mut pixel = Colored::new();
            pixel.set(x, y);
            self.pixels.insert(tile, pixel);
        }

        self
    }

    /// Sets a colored pixel at the specified coordinates.
    pub fn set_colorful<T>(&mut self, x: T, y: T, colors: Colors) -> &mut Self
    where
        T: Into<f64> + Copy,
    {
        self.set(x, y);
        let tile = Tile::from_xy(x, y);
        if let Some(pixel) = self.pixels.get_mut(&tile) {
            if let Some(foreground) = colors.foreground {
                pixel.set_foregound_color(foreground);
            }
            if let Some(background) = colors.background {
                pixel.set_background_color(background);
            }
        } else {
            unreachable!("pixel not found")
        }

        self
    }

    /// Toggles a pixel at the specified coordinates.
    ///
    /// This method toggles the state of a pixel at the given coordinates.
    /// If a pixel exists, it will be toggled. If no pixel exists, a new one
    /// will be created in the toggled state.
    pub fn toggle<T>(&mut self, x: T, y: T) -> &mut Self
    where
        T: Into<f64> + Copy,
    {
        let tile = self.get_tile(x, y);
        let (x, y) = (round(x), round(y));
        if let Some(pixel) = self.pixels.get_mut(&tile) {
            pixel.toggle(x, y);
        } else {
            let mut pixel = Colored::new();
            pixel.toggle(x, y);
            self.pixels.insert(tile, pixel);
        }

        self
    }

    /// Draws a line between two points.
    pub fn line<T>(&mut self, xy1: (T, T), xy2: (T, T)) -> &mut Self
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

        self
    }

    /// Gets the current size of the canvas.
    pub fn get_size(&self) -> (u32, u32) {
        let ((minx, maxx), (miny, maxy)) = self.bound.get_bound();
        ((maxx - minx + 1) as u32, (maxy - miny + 1) as u32)
    }

    /// Sets the boundary of the canvas.
    ///
    /// By default, the canvas is unbounded and will automatically expand as new pixels are drawn.
    /// However, during animations, this automatic expansion can cause the drawing to shift
    /// as the canvas size changes, which is undesirable.
    ///
    /// Setting a fixed boundary prevents the canvas from growing beyond specified limits.
    /// Note: Drawing outside the boundary will still cause the canvas to expand if needed.
    ///
    /// To make the boundary truly fixed (preventing any expansion), call [`fixed_bound`](Self::fixed_bound).
    pub fn set_bound(&mut self, bound_x: (i32, i32), bound_y: (i32, i32)) {
        self.bound.set_bound(bound_x, bound_y);
    }

    /// Sets whether the canvas boundary is fixed.
    ///
    /// When `is_fixed` is true, the canvas boundary cannot be expanded beyond the current limits.
    /// This is useful for creating fixed-size canvases that won't grow during drawing operations.
    pub fn fixed_bound(&mut self, is_fixed: bool) {
        self.bound.fixed_bound(is_fixed);
    }

    fn get_tile<T>(&mut self, x: T, y: T) -> Tile
    where
        T: Into<f64> + Copy,
    {
        let tile = Tile::from_xy(x, y);
        self.bound.update(tile);
        tile
    }
}

impl Draw for Canvas {
    fn draw(&mut self, mut chunk: Chunk) -> Result<(), DrawErr> {
        for (t, p) in &self.pixels {
            if let Some((x, y)) = self.bound.get_terminal_xy(*t) {
                let x = x as u16;
                let y = y as u16;
                if chunk.is_inside(x, y) {
                    chunk.set(x, y, (*p).into())?;
                }
            }
        }
        Ok(())
    }
}
