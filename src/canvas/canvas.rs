use std::collections::HashMap;
use std::io::Write;

use crossterm::{
    cursor::MoveToNextLine,
    queue,
    style::{Color, Print, ResetColor},
};

use super::bound::Bound;
use super::encoder::encode;
use super::sample::{Sample, Samples};
use super::tile::Tile;
use super::Encoding;
use crate::render::{area::Size, chunk::Chunk, style::Stylized, Draw, DrawErr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaintErr;

/// Implement this for painting on a [`Canvas`], independently of its encoding.
pub trait Paint: Send + 'static {
    fn paint<T>(&self, canvas: &mut Canvas, x: T, y: T) -> Result<(), PaintErr>
    where
        T: Into<f64>;
}

impl<T: Paint + ?Sized> Paint for Box<T> {
    fn paint<N>(&self, canvas: &mut Canvas, x: N, y: N) -> Result<(), PaintErr>
    where
        N: Into<f64>,
    {
        (**self).paint(canvas, x, y)
    }
}

/// A sparse, Cartesian sample canvas encoded into terminal cells when drawn.
///
/// Coordinates grow right and upwards and are rounded to the nearest sample.
/// Bounds and [`get_size`](Self::get_size) use terminal cells. Each cell holds
/// 2×4 samples in [`Encoding::Braille`] or 1×2 in [`Encoding::HalfBlock`]. Colors
/// remain attached to individual samples until encoding; empty samples expose
/// the canvas background, or the destination background when none is specified.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Canvas {
    bound: Bound,
    encoding: Encoding,
    samples: HashMap<Tile, Samples>,
    background: Option<Color>,
}

impl Canvas {
    /// Creates an empty, automatically expanding Braille canvas.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects the sample grid at construction. Encoding cannot change under an
    /// existing drawing; construct a new canvas to render with another encoding.
    #[must_use]
    pub fn with_encoding(encoding: Encoding) -> Self {
        Self {
            encoding,
            ..Self::default()
        }
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Sets the background for uncovered samples, including completely empty
    /// cells inside the bounds.
    /// This does not change any painted sample colors.
    pub fn set_background(&mut self, color: impl Into<Color>) -> &mut Self {
        self.background = Some(color.into());
        self
    }

    /// Restores destination-background inheritance for uncovered samples.
    pub fn clear_background(&mut self) -> &mut Self {
        self.background = None;
        self
    }

    pub fn paint<T: Paint, N: Into<f64>>(
        &mut self,
        target: &T,
        x: N,
        y: N,
    ) -> Result<(), PaintErr> {
        target.paint(self, x.into(), y.into())
    }

    /// Prints to stdout, using the terminal's current raw-mode setting.
    pub fn print(&self) {
        let is_raw = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
        self.print_on(&mut std::io::stdout(), is_raw).unwrap();
    }

    /// Prints the same encoded cells used by [`Draw`].
    pub fn print_on<W: Write>(&self, w: &mut W, is_raw: bool) -> std::io::Result<()> {
        let ((minx, maxx), (miny, maxy)) = self.bound.get_bound();
        for row in (miny..=maxy).rev() {
            for col in minx..=maxx {
                let cell = self.encode_cell(Tile::from(col, row));
                cell.queue(w)?;
                if cell.has_color() {
                    queue!(w, ResetColor)?;
                }
            }
            if is_raw {
                queue!(w, MoveToNextLine(1))?;
            } else {
                queue!(w, Print("\n"))?;
            }
        }
        w.flush()
    }

    /// Clears all samples, retaining bounds, encoding and background.
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// Clears samples and restores automatically expanding bounds, retaining the
    /// selected encoding and background.
    pub fn reset(&mut self) {
        self.bound = Bound::new();
        self.clear();
    }

    /// Occupies a sample with the inherited foreground color.
    pub fn set<T: Into<f64> + Copy>(&mut self, x: T, y: T) -> &mut Self {
        self.set_sample(x, y, Sample { color: None })
    }

    /// Paints one sample, retaining its color until terminal-cell encoding.
    ///
    /// Half-block preserves both sample colors exactly. Braille preserves the
    /// coverage mask of partially painted cells and averages their ink colors;
    /// fully colored cells fit their eight samples to two terminal colors.
    /// Palette colors are preserved when possible; mixing uses the conventional
    /// xterm palette. Use RGB colors when exact color fitting is needed.
    pub fn set_color<T: Into<f64> + Copy>(
        &mut self,
        x: T,
        y: T,
        color: impl Into<Color>,
    ) -> &mut Self {
        self.set_sample(
            x,
            y,
            Sample {
                color: Some(color.into()),
            },
        )
    }

    fn set_sample<T: Into<f64> + Copy>(&mut self, x: T, y: T, sample: Sample) -> &mut Self {
        if let Some((tile, index)) = self.location(x, y) {
            self.bound.update(tile);
            self.samples.entry(tile).or_default().0[index] = Some(sample);
        }
        self
    }

    /// Removes a sample without changing the canvas bounds.
    pub fn unset<T: Into<f64> + Copy>(&mut self, x: T, y: T) -> &mut Self {
        if let Some((tile, index)) = self.location(x, y) {
            if let Some(samples) = self.samples.get_mut(&tile) {
                samples.0[index] = None;
                if samples.0.iter().all(Option::is_none) {
                    self.samples.remove(&tile);
                }
            }
        }
        self
    }

    /// Removes occupied samples, or sets empty samples with inherited foreground.
    pub fn toggle<T: Into<f64> + Copy>(&mut self, x: T, y: T) -> &mut Self {
        if let Some((tile, index)) = self.location(x, y) {
            if self
                .samples
                .get(&tile)
                .is_some_and(|s| s.0[index].is_some())
            {
                self.unset(x, y);
            } else {
                self.set(x, y);
            }
        }
        self
    }

    /// Draws a line in sample coordinates. Non-finite coordinates are ignored;
    /// fixed bounds clip the line before rasterization.
    pub fn line<T: Into<f64>>(&mut self, from: (T, T), to: (T, T)) -> &mut Self {
        let (mut x, mut y) = (from.0.into(), from.1.into());
        let (end_x, end_y) = (to.0.into(), to.1.into());
        if ![x, y, end_x, end_y].into_iter().all(f64::is_finite) {
            return self;
        }
        x = x.round();
        y = y.round();
        let (end_x, end_y) = (end_x.round(), end_y.round());
        let (dx, dy) = (end_x - x, end_y - y);
        if !dx.is_finite() || !dy.is_finite() {
            return self;
        }
        let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
        if self.bound.is_fixed() {
            let ((minx, maxx), (miny, maxy)) = self.bound.get_bound();
            let (w, h) = self.encoding.samples_per_cell();
            let (w, h) = (f64::from(w), f64::from(h));
            for (p, q) in [
                (-dx, x - f64::from(minx) * w),
                (dx, (f64::from(maxx) + 1.0) * w - 1.0 - x),
                (-dy, y - f64::from(miny) * h),
                (dy, (f64::from(maxy) + 1.0) * h - 1.0 - y),
            ] {
                if p == 0.0 {
                    if q < 0.0 {
                        return self;
                    }
                } else {
                    let t = q / p;
                    if p < 0.0 {
                        enter = enter.max(t);
                    } else {
                        leave = leave.min(t);
                    }
                    if enter > leave {
                        return self;
                    }
                }
            }
        }
        let (end_x, end_y) = ((x + leave * dx).round(), (y + leave * dy).round());
        x = (x + enter * dx).round();
        y = (y + enter * dy).round();
        if ![x, y, end_x, end_y]
            .into_iter()
            .all(|v| v >= f64::from(i32::MIN) && v <= f64::from(i32::MAX))
        {
            return self;
        }
        let steps = (end_x - x).abs().max((end_y - y).abs()) as usize;
        if steps == 0 {
            return self.set(x, y);
        }
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            self.set(x + (end_x - x) * t, y + (end_y - y) * t);
        }
        self
    }

    /// Current terminal-cell dimensions, independent of sample density.
    pub fn get_size(&self) -> (u32, u32) {
        let ((minx, maxx), (miny, maxy)) = self.bound.get_bound();
        let size = |min: i32, max: i32| {
            u32::try_from(i64::from(max) - i64::from(min) + 1).unwrap_or(u32::MAX)
        };
        (size(minx, maxx), size(miny, maxy))
    }

    /// Sets inclusive bounds in terminal cells. These expand with drawing until
    /// [`fixed_bound(true)`](Self::fixed_bound) is called.
    pub fn set_bound(&mut self, bound_x: (i32, i32), bound_y: (i32, i32)) {
        assert!(
            bound_x.0 <= bound_x.1 && bound_y.0 <= bound_y.1,
            "invalid canvas bounds"
        );
        self.bound.set_bound(bound_x, bound_y);
    }

    /// Fixed bounds reject out-of-bounds samples before allocating storage.
    pub fn fixed_bound(&mut self, is_fixed: bool) {
        self.bound.fixed_bound(is_fixed);
    }

    fn location<T: Into<f64> + Copy>(&self, x: T, y: T) -> Option<(Tile, usize)> {
        let (x, y) = (x.into().round(), y.into().round());
        if ![x, y]
            .into_iter()
            .all(|v| v.is_finite() && v >= f64::from(i32::MIN) && v <= f64::from(i32::MAX))
        {
            return None;
        }
        let (x, y) = (x as i32, y as i32);
        let (w, h) = self.encoding.samples_per_cell();
        let (w, h) = (i32::from(w), i32::from(h));
        let tile = Tile::from(x.div_euclid(w), y.div_euclid(h));
        if self.bound.is_fixed() && !self.bound.tile_inside(tile) {
            return None;
        }
        Some((tile, (y.rem_euclid(h) * w + x.rem_euclid(w)) as usize))
    }

    fn encode_cell(&self, tile: Tile) -> Stylized {
        encode(
            self.encoding,
            self.samples.get(&tile).unwrap_or(&Samples::default()),
            self.background,
        )
    }

    fn draw_cell(&self, chunk: &mut Chunk, x: u16, y: u16, tile: Tile) -> Result<(), DrawErr> {
        let mut cell = self.encode_cell(tile);
        // Block characters can be double-width under a CJK text policy. Use
        // one-column Braille equivalents so graphics retain their cell geometry.
        let width = cell.width_with(chunk.text_metrics(), x as usize);
        if width != 1 {
            let symbol = match cell.as_char() {
                Some('▀') => '\u{281b}',
                Some('▄') => '\u{28e4}',
                Some('█') => '\u{28ff}',
                _ => {
                    return Err(DrawErr::content_too_wide(
                        width as u16,
                        chunk.area().width(),
                        x,
                    ))
                }
            };
            cell.set_char(symbol);
        }
        chunk.set_forced(x, y, cell)?;
        Ok(())
    }
}

impl Draw for Canvas {
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        if self.background.is_some() {
            let ((minx, _), (_, maxy)) = self.bound.get_bound();
            let (width, height) = self.get_size();
            for y in 0..height.min(u32::from(chunk.area().height())) {
                for x in 0..width.min(u32::from(chunk.area().width())) {
                    let tile = Tile::from(
                        (i64::from(minx) + i64::from(x)) as i32,
                        (i64::from(maxy) - i64::from(y)) as i32,
                    );
                    self.draw_cell(&mut chunk, x as u16, y as u16, tile)?;
                }
            }
        } else {
            for tile in self.samples.keys() {
                if let Some((x, y)) = self.bound.get_terminal_xy(*tile) {
                    if let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) {
                        if chunk.is_inside(x, y) {
                            self.draw_cell(&mut chunk, x, y, *tile)?;
                        }
                    }
                }
            }
        }
        Ok(chunk.area().size())
    }
}
