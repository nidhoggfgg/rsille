//! Sparse sample storage, painting operations and terminal output.
//!
//! Samples use Cartesian coordinates; bounds and output sizes use terminal
//! cells. The selected encoding determines how samples are grouped into cells.

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

/// An error reported by a [`Paint`] implementation.
///
/// This marker carries no additional error details. A failed paint operation
/// may have modified the canvas; changes are not rolled back automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaintErr;

/// Implement this for painting on a [`Canvas`], independently of its encoding.
///
/// Implementations choose how to interpret the supplied sample-space origin.
/// Boxed painters also implement this trait.
///
/// # Examples
///
/// ```
/// use rsille::canvas::{Canvas, Paint, PaintErr};
///
/// struct Cross;
///
/// impl Paint for Cross {
///     fn paint<T: Into<f64>>(
///         &self, canvas: &mut Canvas, x: T, y: T,
///     ) -> Result<(), PaintErr> {
///         let (x, y) = (x.into(), y.into());
///         canvas.line((x - 1.0, y), (x + 1.0, y));
///         canvas.line((x, y - 1.0), (x, y + 1.0));
///         Ok(())
///     }
/// }
///
/// let mut canvas = Canvas::new();
/// canvas.paint(&Cross, 4, 4).unwrap();
/// ```
pub trait Paint: Send + 'static {
    /// Paints onto `canvas` at the sample-space origin `(x, y)`.
    ///
    /// # Errors
    ///
    /// Returns [`PaintErr`] if the implementation cannot complete the drawing.
    /// The implementation determines the failure conditions and may leave a
    /// partial drawing on the canvas.
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
/// 2×4 samples in [`Encoding::Braille`], 1×2 in [`Encoding::HalfBlock`], or
/// 2×2 in [`Encoding::Quadrant`]. Colors
/// remain attached to individual samples until encoding; empty samples expose
/// the canvas background, or the destination background when none is specified.
///
/// # Coordinates and bounds
///
/// Sample coordinates are rounded with halfway values away from zero. Point
/// operations ignore non-finite values and rounded values outside the `i32`
/// range. Negative coordinates are supported: each cell groups samples using
/// Euclidean division by its sample width and height.
///
/// A new canvas has a single cell at the origin. Bounds expand as samples are
/// added, unless frozen with [`fixed_bound`](Self::fixed_bound). Removing samples
/// does not shrink the bounds. Use [`reset`](Self::reset) to restore the initial
/// bounds, or [`set_bound`](Self::set_bound) to choose a visible region.
///
/// # Output
///
/// [`Draw`] places the top-left bound at the destination chunk's origin and clips
/// output to the chunk. Without an explicit background, entirely empty cells
/// leave the destination untouched. With a background, all cells inside the
/// visible bounds are written. Block glyphs that the chunk considers wide are
/// replaced with single-column Braille equivalents.
///
/// [`print_on`](Self::print_on) writes every cell inside the bounds, including
/// empty cells, directly to a writer without applying a text-width policy.
///
/// # Examples
///
/// ```
/// use rsille::canvas::{Canvas, Encoding};
///
/// let mut canvas = Canvas::with_encoding(Encoding::HalfBlock);
/// canvas.set_bound((0, 2), (0, 0)); // Three columns, one terminal row.
/// canvas.fixed_bound(true);
/// canvas.line((0, 0), (2, 0));
/// assert_eq!(canvas.get_size(), (3, 1));
///
/// let mut output = Vec::new();
/// canvas.print_on(&mut output, false).unwrap();
/// assert_eq!(String::from_utf8(output).unwrap(), "▄▄▄\n");
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Canvas {
    bound: Bound,
    encoding: Encoding,
    samples: HashMap<Tile, Samples>,
    background: Option<Color>,
}

impl Canvas {
    /// Creates an empty, automatically expanding Braille canvas.
    ///
    /// The initial bounds contain only the origin cell, so
    /// [`get_size`](Self::get_size) returns `(1, 1)` even before painting.
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

    /// Returns the encoding selected when this canvas was constructed.
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Sets the background for uncovered samples, including completely empty
    /// cells inside the bounds.
    /// This does not change any painted sample colors.
    /// Fully colored cells may use both terminal colors to represent their
    /// samples, overriding this background within those cells.
    pub fn set_background(&mut self, color: impl Into<Color>) -> &mut Self {
        self.background = Some(color.into());
        self
    }

    /// Restores destination-background inheritance for uncovered samples.
    pub fn clear_background(&mut self) -> &mut Self {
        self.background = None;
        self
    }

    /// Paints `target` at `(x, y)` by delegating to [`Paint::paint`].
    ///
    /// Coordinates are passed as floating-point sample coordinates without
    /// rounding or clipping here; the painter decides how to use them.
    ///
    /// # Errors
    ///
    /// Propagates the painter's [`PaintErr`]. Any changes already made by the
    /// painter remain on the canvas.
    pub fn paint<T: Paint, N: Into<f64>>(
        &mut self,
        target: &T,
        x: N,
        y: N,
    ) -> Result<(), PaintErr> {
        target.paint(self, x.into(), y.into())
    }

    /// Prints to stdout, using the terminal's current raw-mode setting.
    ///
    /// Assumes cooked mode if querying raw mode fails. Use
    /// [`print_on`](Self::print_on) to handle output errors yourself.
    ///
    /// # Panics
    ///
    /// Panics if writing or flushing stdout fails.
    pub fn print(&self) {
        let is_raw = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
        self.print_on(&mut std::io::stdout(), is_raw).unwrap();
    }

    /// Writes the encoded canvas from top to bottom and flushes `w`.
    ///
    /// Every cell inside the bounds is written, including empty cells. Colors
    /// are reset after each colored cell. Each row, including the last, ends
    /// with a newline when `is_raw` is `false`, or a terminal command to move
    /// to the start of the next line when `is_raw` is `true`. This method does
    /// not change the terminal's raw-mode setting.
    ///
    /// Uses the same cell encoder as [`Draw`], but does not perform its
    /// text-width-dependent glyph substitution.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing commands or flushing the writer fails.
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
    /// The restored bounds contain only the origin cell, with size `(1, 1)`.
    pub fn reset(&mut self) {
        self.bound = Bound::new();
        self.clear();
    }

    /// Occupies a sample with the inherited foreground color.
    ///
    /// Replaces any color previously assigned to this sample. Within a Braille
    /// or Quadrant cell, explicitly colored neighbors can determine the shared
    /// foreground. Coordinates follow the rules on [`Canvas`]; samples outside
    /// fixed bounds are ignored.
    pub fn set<T: Into<f64> + Copy>(&mut self, x: T, y: T) -> &mut Self {
        self.set_sample(x, y, Sample { color: None })
    }

    /// Paints one sample, retaining its color until terminal-cell encoding.
    ///
    /// Half-block preserves both sample colors exactly. Braille and Quadrant
    /// preserve the coverage mask of partially painted cells and average their ink colors;
    /// fully colored cells fit their samples to two terminal colors.
    /// Palette colors are preserved when possible; mixing uses the conventional
    /// xterm palette. Use RGB colors when exact color fitting is needed.
    ///
    /// Replaces any existing sample at the rounded coordinates. Coordinates
    /// follow the rules on [`Canvas`]; samples outside fixed bounds are ignored.
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

    /// Stores a valid sample and expands unfixed bounds to include its cell.
    fn set_sample<T: Into<f64> + Copy>(&mut self, x: T, y: T, sample: Sample) -> &mut Self {
        if let Some((tile, index)) = self.location(x, y) {
            self.bound.update(tile);
            self.samples.entry(tile).or_default().0[index] = Some(sample);
        }
        self
    }

    /// Removes a sample without changing the canvas bounds.
    ///
    /// Empty samples, invalid coordinates and samples outside fixed bounds are
    /// ignored. Uncovered space uses the canvas or destination background.
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
    ///
    /// Removing a colored sample discards its color. Invalid coordinates and
    /// samples outside fixed bounds are ignored.
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
    ///
    /// Endpoints are rounded and included in the drawing. Samples are painted
    /// with [`set`](Self::set), replacing any existing sample colors. A line
    /// whose rounded endpoints coincide paints one sample. The line is ignored
    /// if its coordinate differences are non-finite or its clipped, rounded
    /// endpoints cannot be represented as `i32`.
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
    ///
    /// Returns `(width, height)` from the inclusive bounds, including empty
    /// cells. Each dimension saturates at [`u32::MAX`].
    pub fn get_size(&self) -> (u32, u32) {
        let ((minx, maxx), (miny, maxy)) = self.bound.get_bound();
        let size = |min: i32, max: i32| {
            u32::try_from(i64::from(max) - i64::from(min) + 1).unwrap_or(u32::MAX)
        };
        (size(minx, maxx), size(miny, maxy))
    }

    /// Sets inclusive bounds in terminal cells. These expand with drawing until
    /// [`fixed_bound(true)`](Self::fixed_bound) is called.
    ///
    /// Does not change whether the bounds are fixed or remove stored samples.
    /// Samples outside the new bounds become visible again if the bounds later
    /// include them. Each pair is `(minimum, maximum)`; negative bounds are valid.
    ///
    /// # Panics
    ///
    /// Panics if either minimum is greater than its corresponding maximum.
    pub fn set_bound(&mut self, bound_x: (i32, i32), bound_y: (i32, i32)) {
        assert!(
            bound_x.0 <= bound_x.1 && bound_y.0 <= bound_y.1,
            "invalid canvas bounds"
        );
        self.bound.set_bound(bound_x, bound_y);
    }

    /// Fixed bounds reject out-of-bounds samples before allocating storage.
    ///
    /// Pass `true` to freeze the current bounds, or `false` to allow subsequent
    /// painting to expand them. Existing samples are retained in either case.
    pub fn fixed_bound(&mut self, is_fixed: bool) {
        self.bound.fixed_bound(is_fixed);
    }

    /// Resolves rounded coordinates to a tile and Cartesian sample-slot index.
    ///
    /// Rejects non-finite or unrepresentable coordinates and tiles excluded by
    /// fixed bounds before any sample storage is allocated.
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

    /// Encodes a tile, treating missing storage as entirely uncovered samples.
    fn encode_cell(&self, tile: Tile) -> Stylized {
        encode(
            self.encoding,
            self.samples.get(&tile).unwrap_or(&Samples::default()),
            self.background,
        )
    }

    /// Writes one encoded tile, substituting Braille for wide block glyphs.
    ///
    /// Returns a drawing error if a glyph cannot fit or the chunk rejects the
    /// destination coordinates.
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
                Some('▖') => '\u{2844}',
                Some('▗') => '\u{28a0}',
                Some('▘') => '\u{2803}',
                Some('▌') => '\u{2847}',
                Some('▚') => '\u{28a3}',
                Some('▙') => '\u{28e7}',
                Some('▝') => '\u{2818}',
                Some('▞') => '\u{285c}',
                Some('▐') => '\u{28b8}',
                Some('▟') => '\u{28fc}',
                Some('▛') => '\u{285f}',
                Some('▜') => '\u{28bb}',
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

/// Renders into a chunk and returns the chunk's size, rather than the canvas size.
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
