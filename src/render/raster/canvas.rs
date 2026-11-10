//! Opaque RGB drawing resources and protocol-independent presentation sampling.
//!
//! Canvas coordinates are signed integer pixel coordinates, with `(0, 0)` at
//! the top left and positive y pointing down. All drawing is clipped to the
//! canvas. Snapshots share storage until a write actually changes content;
//! presentation never resizes or copies the source pixel buffer.

use std::fmt;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crossterm::style::{Attributes, Color, Colors};

use crate::render::area::Size;
use crate::render::style::{Style, Stylized};

/// An opaque, eight-bit-per-channel RGB color; the default is black.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// Maximum source pixel count (48 MiB of RGB storage per distinct snapshot).
pub const MAX_RASTER_PIXELS: usize = 16 * 1024 * 1024;

/// Failure to construct a raster resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterError {
    /// Both dimensions must be nonzero.
    EmptyDimensions,
    /// The dimensions overflow addressable storage or exceed [`MAX_RASTER_PIXELS`].
    TooManyPixels,
    /// The allocator could not reserve the requested pixel storage.
    AllocationFailed,
}

impl fmt::Display for RasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyDimensions => "raster dimensions must be nonzero",
            Self::TooManyPixels => "raster dimensions exceed the pixel resource limit",
            Self::AllocationFailed => "could not allocate raster pixels",
        })
    }
}

impl std::error::Error for RasterError {}

// Tokens are shared across all lineages, not incremented on each clone. Two
// branches can therefore never acquire the same (id, version) after mutation.
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

fn next_token() -> u64 {
    NEXT_TOKEN
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .expect("raster identity/version space exhausted")
}

/// A fixed-size, opaque RGB canvas with cheap copy-on-write snapshots.
///
/// Cloning is O(1). The first content-changing write while pixels are shared
/// copies the source buffer; later writes to uniquely owned storage do not.
/// No-op writes preserve the version and do not detach storage. Equality is an
/// O(1) comparison of resource identity and version, not a pixel comparison:
/// independently constructed canvases are unequal even with identical pixels.
///
/// Drawing is not antialiased. Work is bounded by canvas dimensions, not the
/// magnitude of offscreen coordinates or radii. Mutations are infallible APIs:
/// COW allocation failure follows the standard allocator failure behavior.
#[derive(Clone, Debug)]
pub struct RasterCanvas {
    width: u32,
    height: u32,
    background: Rgb,
    pixels: Arc<Vec<Rgb>>,
    id: u64,
    version: u64,
}

impl PartialEq for RasterCanvas {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.version == other.version
    }
}

impl Eq for RasterCanvas {}

impl RasterCanvas {
    /// Allocates a nonempty canvas filled with `background`.
    ///
    /// Dimensions are checked before allocation; at most [`MAX_RASTER_PIXELS`]
    /// pixels are accepted. Dimensions never change after construction.
    pub fn new(width: u32, height: u32, background: Rgb) -> Result<Self, RasterError> {
        if width == 0 || height == 0 {
            return Err(RasterError::EmptyDimensions);
        }
        let len = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .filter(|&len| len <= MAX_RASTER_PIXELS)
            .ok_or(RasterError::TooManyPixels)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(len)
            .map_err(|_| RasterError::AllocationFailed)?;
        pixels.resize(len, background);
        Ok(Self {
            width,
            height,
            background,
            pixels: Arc::new(pixels),
            id: next_token(),
            version: 0,
        })
    }

    /// Source width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Source height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The construction color, or the color of the most recent `clear`.
    pub fn background(&self) -> Rgb {
        self.background
    }

    /// Immutable row-major RGB pixels, without padding or alpha.
    pub fn pixels(&self) -> &[Rgb] {
        self.pixels.as_slice()
    }

    /// Stable resource lineage identity, retained by clones and mutations.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Opaque content revision, initially zero and changed on actual mutation.
    ///
    /// Revisions increase but need not be consecutive. Divergent clones always
    /// receive different revisions. Exhausting the process-wide u64 token space
    /// panics rather than allowing cache identity collisions.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Overwrites a pixel. Out-of-bounds coordinates are a no-op.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Rgb) {
        self.painter().pixel(i64::from(x), i64::from(y), color);
    }

    /// Replaces every pixel and the stored background with `color`.
    ///
    /// A change to the background metadata alone also changes the version.
    pub fn clear(&mut self, color: Rgb) {
        let mut painter = self.painter();
        let len = painter.canvas.pixels.len();
        painter.span(0, len, color);
        if painter.canvas.background != color {
            if !painter.changed {
                painter.canvas.version = next_token();
            }
            painter.canvas.background = color;
        }
    }

    /// Draws a line including both endpoints, clipped to the canvas.
    ///
    /// The dominant axis advances one pixel at a time; the minor coordinate is
    /// rounded to nearest (ties toward the ending minor coordinate after sorting
    /// by the dominant axis). Endpoints may be supplied in either order.
    /// At most `max(width, height)` candidates are visited, even for i32 extremes.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgb) {
        let (mut x0, mut y0, mut x1, mut y1) =
            (i64::from(x0), i64::from(y0), i64::from(x1), i64::from(y1));
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }
        let limit = i64::from(if steep { self.height } else { self.width });
        let start = x0.max(0);
        let end = x1.min(limit - 1);
        let dx = x1 - x0;
        let dy = y1 - y0;
        let mut painter = self.painter();
        for major in start..=end {
            // i32 endpoint differences can span u32::MAX; their product may
            // exceed i64 even though every resulting coordinate fits in i64.
            let minor = if dx == 0 {
                y0
            } else {
                y0 + dy.signum()
                    * ((i128::from(major - x0) * i128::from(dy.abs()) + i128::from(dx / 2))
                        / i128::from(dx)) as i64
            };
            if steep {
                painter.pixel(minor, major, color);
            } else {
                painter.pixel(major, minor, color);
            }
        }
    }

    /// Draws the outline of `[x, x + width) × [y, y + height)`.
    ///
    /// All arguments are signed pixel coordinates/extents. Non-positive extents
    /// are a no-op. Arithmetic is widened before addition; clipping does not
    /// introduce new outline edges at the canvas boundary. Work is O(width of
    /// canvas + height of canvas), plus any first-write COW copy.
    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Rgb) {
        if width <= 0 || height <= 0 {
            return;
        }
        let (left, top) = (i64::from(x), i64::from(y));
        let right = left + i64::from(width) - 1;
        let bottom = top + i64::from(height) - 1;
        let last_y = bottom.min(i64::from(self.height) - 1);
        let mut painter = self.painter();
        painter.row(left, right, top, color);
        painter.row(left, right, bottom, color);
        for row in top.max(0)..=last_y {
            painter.pixel(left, row, color);
            painter.pixel(right, row, color);
        }
    }

    /// Fills `[x, x + width) × [y, y + height)`, clipped to the canvas.
    ///
    /// Non-positive signed extents are a no-op. Work is bounded by the source
    /// pixel count, regardless of the requested rectangle's offscreen extent.
    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Rgb) {
        if width <= 0 || height <= 0 {
            return;
        }
        let (left, top) = (i64::from(x), i64::from(y));
        let right = left + i64::from(width) - 1;
        let bottom = (top + i64::from(height) - 1).min(i64::from(self.height) - 1);
        let mut painter = self.painter();
        for row in top.max(0)..=bottom {
            painter.row(left, right, row, color);
        }
    }

    /// Draws a circle centered at `(x, y)` with a signed pixel radius.
    ///
    /// Negative radii are a no-op; radius zero draws one pixel. The outline is
    /// the union of the nearest integer boundary points on each row and column,
    /// not an antialiased stroke. Only visible rows/columns are visited, bounding
    /// work by O(canvas width + canvas height), even for `i32::MAX` radii.
    pub fn circle(&mut self, x: i32, y: i32, radius: i32, color: Rgb) {
        if radius < 0 {
            return;
        }
        let (x, y, radius) = (i64::from(x), i64::from(y), i64::from(radius));
        let right = (x + radius).min(i64::from(self.width) - 1);
        let bottom = (y + radius).min(i64::from(self.height) - 1);
        let mut painter = self.painter();
        for column in (x - radius).max(0)..=right {
            let extent = circle_extent(radius, column - x, true);
            painter.pixel(column, y - extent, color);
            painter.pixel(column, y + extent, color);
        }
        for row in (y - radius).max(0)..=bottom {
            let extent = circle_extent(radius, row - y, true);
            painter.pixel(x - extent, row, color);
            painter.pixel(x + extent, row, color);
        }
    }

    /// Fills integer pixel centers whose squared distance is at most `radius²`.
    ///
    /// Negative radii are a no-op; radius zero draws one pixel. Clipped row spans
    /// bound work by the canvas pixel count, not the area of an offscreen circle.
    pub fn fill_circle(&mut self, x: i32, y: i32, radius: i32, color: Rgb) {
        if radius < 0 {
            return;
        }
        let (x, y, radius) = (i64::from(x), i64::from(y), i64::from(radius));
        let bottom = (y + radius).min(i64::from(self.height) - 1);
        let mut painter = self.painter();
        for row in (y - radius).max(0)..=bottom {
            let extent = circle_extent(radius, row - y, false);
            painter.row(x - extent, x + extent, row, color);
        }
    }

    fn painter(&mut self) -> Painter<'_> {
        Painter {
            canvas: self,
            changed: false,
        }
    }
}

fn circle_extent(radius: i64, offset: i64, nearest: bool) -> i64 {
    let squared = (radius * radius - offset * offset) as u64;
    let floor = squared.isqrt();
    // Integer comparison against (floor + 0.5)^2, without floating point.
    (floor + u64::from(nearest && squared - floor * floor > floor)) as i64
}

struct Painter<'a> {
    canvas: &'a mut RasterCanvas,
    changed: bool,
}

impl Painter<'_> {
    fn span(&mut self, start: usize, end: usize, color: Rgb) {
        if self.canvas.pixels[start..end]
            .iter()
            .all(|&old| old == color)
        {
            return;
        }
        let pixels = if self.changed {
            Arc::get_mut(&mut self.canvas.pixels).expect("painter owns detached pixels")
        } else {
            self.canvas.version = next_token();
            self.changed = true;
            Arc::make_mut(&mut self.canvas.pixels)
        };
        pixels[start..end].fill(color);
    }

    fn pixel(&mut self, x: i64, y: i64, color: Rgb) {
        if x >= 0 && y >= 0 && x < i64::from(self.canvas.width) && y < i64::from(self.canvas.height)
        {
            let index = y as usize * self.canvas.width as usize + x as usize;
            self.span(index, index + 1, color);
        }
    }

    fn row(&mut self, left: i64, right: i64, y: i64, color: Rgb) {
        if y < 0 || y >= i64::from(self.canvas.height) {
            return;
        }
        let left = left.max(0);
        let right = right.min(i64::from(self.canvas.width) - 1);
        if left <= right {
            let row = y as usize * self.canvas.width as usize;
            self.span(row + left as usize, row + right as usize + 1, color);
        }
    }
}

/// How the source is scaled into the full target rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RasterScale {
    /// Preserve aspect ratio, filling unused pixels with the presentation background.
    #[default]
    Fit,
    /// Scale each axis independently to fill the target.
    Stretch,
}

/// Placement of any unused space along an axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RasterAlign {
    /// Place the image against the top or left edge.
    Start,
    /// Center the image; an odd extra pixel goes to the bottom or right.
    #[default]
    Center,
    /// Place the image against the bottom or right edge.
    End,
}

impl RasterAlign {
    fn offset(self, unused: u64) -> u64 {
        match self {
            Self::Start => 0,
            Self::Center => unused / 2,
            Self::End => unused,
        }
    }
}

/// Character-only approximation, independent of the source raster resolution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RasterFallback {
    /// One upper and one lower RGB sample per cell.
    #[default]
    HalfBlock,
    /// Eight samples per cell, approximated by two RGB colors and a dot mask.
    Braille,
}

/// Presentation settings. Defaults: black letterbox, centered fit, half blocks.
///
/// `background` is independent of the source canvas background. Both are opaque.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RasterOptions {
    /// Opaque letterbox and invalid-sample color.
    pub background: Rgb,
    /// Source-to-target scaling policy.
    pub scale: RasterScale,
    /// Horizontal placement within the full target.
    pub align_x: RasterAlign,
    /// Vertical placement within the full target.
    pub align_y: RasterAlign,
    /// Character representation when native raster output is unavailable.
    pub fallback: RasterFallback,
}

/// A source snapshot and the original offset of one cell in its full target.
///
/// `x` and `y` are NOT the cell's current screen position. Clipping, scrolling,
/// and offscreen blitting must preserve all this metadata, changing only where
/// the cell is stored. Sampling then crops the original presentation instead of
/// rescaling a visible fragment. Cloning this value never copies source pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterCell {
    /// Immutable-by-convention COW source snapshot.
    pub canvas: RasterCanvas,
    /// Full, unclipped presentation size in terminal cells.
    pub target: Size,
    /// Original column within `target`.
    pub x: u16,
    /// Original row within `target`.
    pub y: u16,
    /// Scaling, placement, and fallback settings.
    pub options: RasterOptions,
}

impl RasterCell {
    /// Samples a pixel LOCAL to this cell, using actual pixel dimensions per cell.
    ///
    /// Scaling uses the full target's physical pixel size and nearest-neighbor
    /// sampling at destination pixel centers. Fit dimensions are rounded down to
    /// integer pixels (minimum one), with explicit opaque letterboxing. Zero
    /// metrics, empty targets, invalid cell offsets, and local coordinates beyond
    /// the cell all return `options.background`. No allocations are performed.
    pub fn sample(&self, pixel_x: u32, pixel_y: u32, cell_width: u16, cell_height: u16) -> Rgb {
        let background = self.options.background;
        if cell_width == 0
            || cell_height == 0
            || self.x >= self.target.width
            || self.y >= self.target.height
            || pixel_x >= u32::from(cell_width)
            || pixel_y >= u32::from(cell_height)
        {
            return background;
        }
        let target_width = u64::from(self.target.width) * u64::from(cell_width);
        let target_height = u64::from(self.target.height) * u64::from(cell_height);
        let source_width = u64::from(self.canvas.width);
        let source_height = u64::from(self.canvas.height);
        let (width, height) = match self.options.scale {
            RasterScale::Stretch => (target_width, target_height),
            RasterScale::Fit => {
                if target_width * source_height <= target_height * source_width {
                    (
                        target_width,
                        (target_width * source_height / source_width).max(1),
                    )
                } else {
                    (
                        (target_height * source_width / source_height).max(1),
                        target_height,
                    )
                }
            }
        };
        let left = self.options.align_x.offset(target_width - width);
        let top = self.options.align_y.offset(target_height - height);
        let x = u64::from(self.x) * u64::from(cell_width) + u64::from(pixel_x);
        let y = u64::from(self.y) * u64::from(cell_height) + u64::from(pixel_y);
        if x < left || y < top || x >= left + width || y >= top + height {
            return background;
        }
        let source_x = ((2 * (x - left) + 1) * source_width) / (2 * width);
        let source_y = ((2 * (y - top) + 1) * source_height) / (2 * height);
        self.canvas.pixels[(source_y * source_width + source_x) as usize]
    }

    /// Produces one opaque styled terminal cell, without copying the source.
    ///
    /// HalfBlock samples a 1×2 grid, Braille a 2×4 grid. Both assume a terminal
    /// cell aspect ratio of 1:2; these grids are NOT measured terminal pixels.
    /// Braille uses a deterministic two-color approximation: choose the farthest
    /// RGB sample pair, assign each sample to its nearest seed (ties to the first),
    /// then average each group. It cannot retain eight independent sample colors.
    /// Both foreground/background and empty attributes are explicit, including
    /// uniform cells and letterboxing. Glyph widths follow the renderer's normal
    /// Unicode width policy.
    pub fn fallback(&self) -> Stylized {
        match self.options.fallback {
            RasterFallback::HalfBlock => {
                styled('▀', self.sample(0, 0, 1, 2), self.sample(0, 1, 1, 2))
            }
            RasterFallback::Braille => {
                let samples = std::array::from_fn(|index| {
                    self.sample((index % 2) as u32, (index / 2) as u32, 2, 4)
                });
                braille(samples)
            }
        }
    }
}

fn styled(symbol: char, foreground: Rgb, background: Rgb) -> Stylized {
    let color = |Rgb(r, g, b)| Color::Rgb { r, g, b };
    Stylized::new(
        symbol,
        Style::with_both(
            Colors {
                foreground: Some(color(foreground)),
                background: Some(color(background)),
            },
            Attributes::default(),
        ),
    )
}

fn distance(a: Rgb, b: Rgb) -> u32 {
    let r = i32::from(a.0) - i32::from(b.0);
    let g = i32::from(a.1) - i32::from(b.1);
    let b = i32::from(a.2) - i32::from(b.2);
    (r * r + g * g + b * b) as u32
}

fn braille(samples: [Rgb; 8]) -> Stylized {
    let (mut first, mut second, mut farthest) = (0, 0, 0);
    for i in 0..8 {
        for j in i + 1..8 {
            let distance = distance(samples[i], samples[j]);
            if distance > farthest {
                (first, second, farthest) = (i, j, distance);
            }
        }
    }
    if farthest == 0 {
        return styled(' ', samples[0], samples[0]);
    }
    let mut sums = [[0u32; 3]; 2];
    let mut counts = [0u32; 2];
    let mut mask = 0u32;
    let bits = [1, 8, 2, 16, 4, 32, 64, 128];
    for (index, &sample) in samples.iter().enumerate() {
        let group =
            usize::from(distance(sample, samples[second]) < distance(sample, samples[first]));
        counts[group] += 1;
        sums[group][0] += u32::from(sample.0);
        sums[group][1] += u32::from(sample.1);
        sums[group][2] += u32::from(sample.2);
        if group == 1 {
            mask |= bits[index];
        }
    }
    let mean = |group: usize| {
        let average = |channel| ((sums[group][channel] + counts[group] / 2) / counts[group]) as u8;
        Rgb(average(0), average(1), average(2))
    };
    styled(char::from_u32(0x2800 + mask).unwrap(), mean(1), mean(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: Rgb = Rgb(0, 0, 0);
    const RED: Rgb = Rgb(255, 0, 0);
    const BLUE: Rgb = Rgb(0, 0, 255);
    const WHITE: Rgb = Rgb(255, 255, 255);

    fn canvas(width: u32, height: u32) -> RasterCanvas {
        RasterCanvas::new(width, height, BLACK).unwrap()
    }

    fn cell(canvas: RasterCanvas, width: u16, height: u16) -> RasterCell {
        RasterCell {
            canvas,
            target: Size { width, height },
            x: 0,
            y: 0,
            options: RasterOptions::default(),
        }
    }

    fn pixel(canvas: &RasterCanvas, x: usize, y: usize) -> Rgb {
        canvas.pixels()[y * canvas.width() as usize + x]
    }

    fn count(canvas: &RasterCanvas, color: Rgb) -> usize {
        canvas.pixels().iter().filter(|&&p| p == color).count()
    }

    #[test]
    fn validates_dimensions_before_allocating() {
        for (width, height) in [(0, 0), (0, u32::MAX), (u32::MAX, 0)] {
            assert_eq!(
                RasterCanvas::new(width, height, BLACK),
                Err(RasterError::EmptyDimensions)
            );
        }
        for (width, height) in [
            (u32::MAX, u32::MAX),
            (u32::MAX, 1),
            (MAX_RASTER_PIXELS as u32 + 1, 1),
            (4097, 4096),
        ] {
            assert_eq!(
                RasterCanvas::new(width, height, BLACK),
                Err(RasterError::TooManyPixels)
            );
        }
        let image = RasterCanvas::new(3, 2, RED).unwrap();
        assert_eq!((image.width(), image.height()), (3, 2));
        assert_eq!(image.background(), RED);
        assert_eq!(image.pixels(), &[RED; 6]);
        assert_eq!(image.version(), 0);
    }

    #[test]
    fn clones_are_shared_until_content_changes() {
        let original = canvas(3, 2);
        let mut changed = original.clone();
        assert_eq!(original, changed);
        assert!(Arc::ptr_eq(&original.pixels, &changed.pixels));
        changed.set_pixel(2, 1, RED);
        assert_ne!(original, changed);
        assert_eq!(original.id(), changed.id());
        assert!(!Arc::ptr_eq(&original.pixels, &changed.pixels));
        assert_eq!(pixel(&original, 2, 1), BLACK);
        assert_eq!(pixel(&changed, 2, 1), RED);
        let pointer = changed.pixels().as_ptr();
        changed.set_pixel(0, 0, BLUE);
        assert_eq!(pointer, changed.pixels().as_ptr());
    }

    #[test]
    fn divergent_clones_never_collide_in_revision() {
        let original = canvas(2, 2);
        let mut a = original.clone();
        let mut b = original.clone();
        a.set_pixel(0, 0, RED);
        b.set_pixel(0, 0, BLUE);
        assert_ne!(a.version(), b.version());
        assert_ne!(a, b);
        let mut c = a.clone();
        a.set_pixel(1, 1, RED);
        c.set_pixel(1, 1, BLUE);
        assert_ne!(a, c);
        assert_ne!(original, canvas(2, 2));
        // Even converging content does not pretend to be the same revision.
        a.clear(BLACK);
        b.clear(BLACK);
        assert_eq!(a.pixels(), b.pixels());
        assert_ne!(a, b);
    }

    #[test]
    fn mutations_on_threads_have_distinct_versions() {
        let image = canvas(2, 2);
        let threads: Vec<_> = (1..=8)
            .map(|r| {
                let mut image = image.clone();
                std::thread::spawn(move || {
                    image.set_pixel(0, 0, Rgb(r, 0, 0));
                    image.version()
                })
            })
            .collect();
        let versions: std::collections::HashSet<_> =
            threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(versions.len(), 8);
        assert_eq!(image.version(), 0);
    }

    #[test]
    fn no_op_operations_preserve_snapshot_and_storage() {
        let original = canvas(3, 3);
        let mut image = original.clone();
        image.set_pixel(-1, 0, RED);
        image.set_pixel(3, 1, RED);
        image.set_pixel(1, 3, RED);
        image.set_pixel(i32::MIN, i32::MAX, RED);
        image.set_pixel(0, 0, BLACK);
        image.clear(BLACK);
        image.line(0, 0, 2, 2, BLACK);
        image.line(-100, -100, -1, -1, RED);
        image.rect(0, 0, 0, 3, RED);
        image.rect(0, 0, 3, -1, RED);
        image.fill_rect(0, 0, -1, 3, RED);
        image.fill_rect(0, 0, 3, 0, RED);
        image.circle(1, 1, -1, RED);
        image.fill_circle(1, 1, i32::MIN, RED);
        image.circle(1, 1, 1, BLACK);
        image.fill_circle(1, 1, 1, BLACK);
        assert_eq!(image, original);
        assert!(Arc::ptr_eq(&image.pixels, &original.pixels));
    }

    #[test]
    fn clear_updates_pixels_and_background_even_if_only_metadata_changes() {
        let mut image = canvas(2, 2);
        image.fill_rect(0, 0, 2, 2, RED);
        let snapshot = image.clone();
        let version = image.version();
        image.clear(RED);
        assert_eq!(image.pixels(), &[RED; 4]);
        assert_eq!(image.background(), RED);
        assert!(image.version() > version);
        assert!(Arc::ptr_eq(&image.pixels, &snapshot.pixels));
        assert_eq!(snapshot.background(), BLACK);
        image.clear(BLUE);
        assert_eq!(image.pixels(), &[BLUE; 4]);
        assert_eq!(snapshot.pixels(), &[RED; 4]);
    }

    #[test]
    fn lines_include_endpoints_and_are_direction_independent_in_all_octants() {
        for x in -2..=8 {
            for y in -2..=8 {
                let mut forward = canvas(7, 7);
                let mut reverse = canvas(7, 7);
                forward.line(3, 3, x, y, RED);
                reverse.line(x, y, 3, 3, RED);
                assert_eq!(forward.pixels(), reverse.pixels(), "endpoint {x},{y}");
                assert_eq!(pixel(&forward, 3, 3), RED);
                if (0..7).contains(&x) && (0..7).contains(&y) {
                    assert_eq!(pixel(&forward, x as usize, y as usize), RED);
                }
            }
        }
        let mut image = canvas(7, 5);
        image.line(-100, 2, 100, 2, RED);
        assert_eq!(count(&image, RED), 7);
        image.line(4, -100, 4, 100, BLUE);
        assert_eq!(count(&image, BLUE), 5);
    }

    #[test]
    fn clipped_line_preserves_original_rasterization_phase() {
        let mut big = canvas(13, 13);
        let mut small = canvas(5, 5);
        big.line(0, 1, 12, 9, RED);
        small.line(-4, -3, 8, 5, RED);
        for y in 0..5 {
            for x in 0..5 {
                assert_eq!(pixel(&small, x, y), pixel(&big, x + 4, y + 4));
            }
        }
    }

    #[test]
    fn rectangles_use_half_open_extents_and_do_not_add_clip_edges() {
        let mut image = canvas(5, 5);
        image.rect(-1, -1, 4, 4, RED);
        assert_eq!(count(&image, RED), 5);
        assert_eq!(pixel(&image, 0, 0), BLACK);
        assert_eq!(pixel(&image, 2, 0), RED);
        assert_eq!(pixel(&image, 0, 2), RED);
        image.fill_rect(-1, -1, 3, 3, BLUE);
        assert_eq!(count(&image, BLUE), 4);
        image.rect(4, 4, 1, 1, WHITE);
        assert_eq!(count(&image, WHITE), 1);
        image.rect(3, 0, 1, 3, WHITE);
        assert_eq!(count(&image, WHITE), 4);
    }

    #[test]
    fn circle_zero_and_small_radius_geometry() {
        let mut image = canvas(7, 7);
        image.circle(3, 3, 0, RED);
        assert_eq!(count(&image, RED), 1);
        image.clear(BLACK);
        image.circle(3, 3, 1, RED);
        assert_eq!(count(&image, RED), 4);
        assert_eq!(pixel(&image, 3, 3), BLACK);
        image.fill_circle(3, 3, 1, BLUE);
        assert_eq!(count(&image, BLUE), 5);
        image.fill_circle(0, 0, 0, WHITE);
        assert_eq!(pixel(&image, 0, 0), WHITE);
    }

    #[test]
    fn filled_circles_match_exact_distance_with_offscreen_centers() {
        for cx in -3i32..=9 {
            for cy in -3i32..=9 {
                for radius in 0..=8 {
                    let mut image = canvas(7, 7);
                    image.fill_circle(cx, cy, radius, RED);
                    for y in 0..7 {
                        for x in 0..7 {
                            let inside = (x - cx).pow(2) + (y - cy).pow(2) <= radius.pow(2);
                            assert_eq!(
                                pixel(&image, x as usize, y as usize),
                                if inside { RED } else { BLACK },
                                "center {cx},{cy}, radius {radius}, pixel {x},{y}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn outlines_are_symmetric_and_clipping_is_a_crop() {
        for radius in 0..=10 {
            let mut big = canvas(25, 25);
            let mut crop = canvas(7, 7);
            big.circle(12, 12, radius, RED);
            crop.circle(2, 1, radius, RED);
            for y in 0..25 {
                for x in 0..25 {
                    assert_eq!(pixel(&big, x, y), pixel(&big, 24 - x, y));
                    assert_eq!(pixel(&big, x, y), pixel(&big, y, x));
                }
            }
            for y in 0..7 {
                for x in 0..7 {
                    assert_eq!(pixel(&crop, x, y), pixel(&big, x + 10, y + 11));
                }
            }
        }
    }

    #[test]
    fn extreme_coordinates_are_bounded_and_do_not_overflow() {
        let mut image = canvas(5, 5);
        image.line(i32::MIN, i32::MIN, i32::MAX, i32::MAX, RED);
        assert_eq!(count(&image, RED), 5);
        image.line(i32::MIN, i32::MAX, i32::MAX, i32::MIN, BLUE);
        assert_eq!(count(&image, BLUE), 0);
        image.fill_rect(-1, -1, i32::MAX, i32::MAX, BLUE);
        assert_eq!(count(&image, BLUE), 25);
        image.rect(-1, -1, i32::MAX, i32::MAX, RED);
        assert_eq!(count(&image, RED), 0);
        image.rect(i32::MAX, i32::MAX, i32::MAX, i32::MAX, RED);
        image.fill_rect(i32::MIN, i32::MIN, i32::MAX, i32::MAX, RED);
        image.circle(0, 0, i32::MAX, RED);
        assert_eq!(count(&image, RED), 0);
        image.fill_circle(0, 0, i32::MAX, RED);
        assert_eq!(count(&image, RED), 25);
        image.circle(i32::MAX, 0, i32::MAX, WHITE);
        assert_eq!(pixel(&image, 0, 0), WHITE);
        image.fill_circle(i32::MIN, i32::MIN, i32::MAX, BLUE);
        image.circle(i32::MIN, i32::MAX, i32::MAX, BLUE);
        assert_eq!(count(&image, BLUE), 0);
    }

    #[test]
    fn defaults_are_opaque_centered_fit_halfblock() {
        let options = RasterOptions::default();
        assert_eq!(options.background, BLACK);
        assert_eq!(options.scale, RasterScale::Fit);
        assert_eq!(options.align_x, RasterAlign::Center);
        assert_eq!(options.align_y, RasterAlign::Center);
        assert_eq!(options.fallback, RasterFallback::HalfBlock);
        assert_eq!(Rgb::default(), BLACK);
    }

    #[test]
    fn stretch_samples_pixel_centers_using_original_offsets() {
        let mut source = canvas(4, 2);
        for y in 0..2 {
            for x in 0..4 {
                source.set_pixel(x, y, Rgb(x as u8, y as u8, 99));
            }
        }
        let mut sample = cell(source, 2, 1);
        sample.options.scale = RasterScale::Stretch;
        for cell_x in 0..2 {
            sample.x = cell_x;
            for y in 0..2 {
                for x in 0..2 {
                    assert_eq!(
                        sample.sample(x, y, 2, 2),
                        Rgb((cell_x * 2) as u8 + x as u8, y as u8, 99)
                    );
                }
            }
        }
        // A cloned/blitted cell retains its original source mapping.
        let blitted = sample.clone();
        assert_eq!(blitted.sample(0, 1, 2, 2), Rgb(2, 1, 99));
        assert!(Arc::ptr_eq(&sample.canvas.pixels, &blitted.canvas.pixels));
        sample.target = Size::from((1, 1));
        sample.x = 0;
        assert_eq!(sample.sample(0, 0, 2, 1), Rgb(1, 1, 99));
        assert_eq!(sample.sample(1, 0, 2, 1), Rgb(3, 1, 99));
    }

    #[test]
    fn nearest_neighbor_upscales_without_interpolation() {
        let mut source = canvas(2, 1);
        source.set_pixel(0, 0, RED);
        source.set_pixel(1, 0, BLUE);
        let mut sample = cell(source, 1, 1);
        sample.options.scale = RasterScale::Stretch;
        assert_eq!(
            (0..4)
                .map(|x| sample.sample(x, 0, 4, 1))
                .collect::<Vec<_>>(),
            vec![RED, RED, BLUE, BLUE]
        );
    }

    #[test]
    fn fit_uses_physical_pixels_not_character_aspect() {
        let mut sample = cell(RasterCanvas::new(4, 4, RED).unwrap(), 2, 1);
        sample.options.background = BLUE;
        // Two 2x4 cells form a square physical target: there is no letterbox.
        for cell_x in 0..2 {
            sample.x = cell_x;
            for y in 0..4 {
                for x in 0..2 {
                    assert_eq!(sample.sample(x, y, 2, 4), RED);
                }
            }
        }
        // The same layout with 4x4 cells has horizontal letterboxing.
        sample.x = 0;
        assert_eq!(sample.sample(0, 0, 4, 4), BLUE);
        assert_eq!(sample.sample(2, 0, 4, 4), RED);
        sample.x = 1;
        assert_eq!(sample.sample(1, 3, 4, 4), RED);
        assert_eq!(sample.sample(2, 3, 4, 4), BLUE);
    }

    #[test]
    fn fit_alignments_place_odd_letterboxing_on_each_axis() {
        for (align, offset) in [
            (RasterAlign::Start, 0),
            (RasterAlign::Center, 1),
            (RasterAlign::End, 3),
        ] {
            let mut sample = cell(RasterCanvas::new(1, 1, RED).unwrap(), 1, 1);
            sample.options.background = BLUE;
            sample.options.align_x = align;
            sample.options.align_y = align;
            for position in 0..5 {
                let expected = if (offset..offset + 2).contains(&position) {
                    RED
                } else {
                    BLUE
                };
                assert_eq!(sample.sample(position, 0, 5, 2), expected);
                assert_eq!(sample.sample(0, position, 2, 5), expected);
            }
        }
    }

    #[test]
    fn fit_rounds_tiny_images_to_at_least_one_pixel() {
        let mut sample = cell(RasterCanvas::new(100, 1, RED).unwrap(), 1, 1);
        sample.options.background = BLUE;
        assert_eq!(sample.sample(0, 0, 1, 2), RED);
        assert_eq!(sample.sample(0, 1, 1, 2), BLUE);
    }

    #[test]
    fn invalid_samples_return_explicit_background() {
        let mut sample = cell(RasterCanvas::new(1, 1, RED).unwrap(), 1, 1);
        sample.options.background = BLUE;
        for (x, y, width, height) in [
            (0, 0, 0, 2),
            (0, 0, 2, 0),
            (2, 0, 2, 2),
            (0, 2, 2, 2),
            (u32::MAX, u32::MAX, u16::MAX, u16::MAX),
        ] {
            assert_eq!(sample.sample(x, y, width, height), BLUE);
        }
        sample.x = 1;
        assert_eq!(sample.sample(0, 0, 2, 2), BLUE);
        sample.x = 0;
        sample.y = 1;
        assert_eq!(sample.sample(0, 0, 2, 2), BLUE);
        sample.y = 0;
        sample.target = Size::from((0, 0));
        assert_eq!(sample.sample(0, 0, 2, 2), BLUE);
    }

    #[test]
    fn maximum_target_metrics_do_not_overflow() {
        let mut sample = cell(RasterCanvas::new(1, 1, RED).unwrap(), u16::MAX, u16::MAX);
        sample.x = u16::MAX - 1;
        sample.y = u16::MAX - 1;
        assert_eq!(sample.sample(65534, 65534, u16::MAX, u16::MAX), RED);
        sample.options.scale = RasterScale::Stretch;
        assert_eq!(sample.sample(65534, 65534, u16::MAX, u16::MAX), RED);
        sample.canvas = RasterCanvas::new(65536, 1, BLUE).unwrap();
        assert_eq!(sample.sample(65534, 65534, u16::MAX, u16::MAX), BLUE);
        sample.options.scale = RasterScale::Fit;
        assert_eq!(sample.sample(65534, 65534, u16::MAX, u16::MAX), BLACK);
    }

    #[test]
    fn halfblock_sets_both_colors_and_attributes() {
        let mut source = canvas(1, 2);
        source.set_pixel(0, 0, RED);
        source.set_pixel(0, 1, BLUE);
        let sample = cell(source, 1, 1);
        assert_eq!(sample.fallback(), styled('▀', RED, BLUE));
        assert_eq!(sample.fallback().width(), 1);
        let mut empty = sample.clone();
        empty.target = Size::default();
        empty.options.background = WHITE;
        assert_eq!(empty.fallback(), styled('▀', WHITE, WHITE));
    }

    #[test]
    fn braille_dot_mapping_and_opaque_colors() {
        let bits = [1, 8, 2, 16, 4, 32, 64, 128];
        for (index, &bit) in bits.iter().enumerate() {
            let mut source = canvas(2, 4);
            source.set_pixel((index % 2) as i32, (index / 2) as i32, WHITE);
            let mut sample = cell(source, 1, 1);
            sample.options.fallback = RasterFallback::Braille;
            let actual = sample.fallback();
            // The first sample seeds the background, so the first-dot case
            // uses an equivalent inverted mask and inverted colors.
            let (mask, fg, bg) = if index == 0 {
                (255 ^ bit, BLACK, WHITE)
            } else {
                (bit, WHITE, BLACK)
            };
            assert_eq!(
                actual,
                styled(char::from_u32(0x2800 + mask).unwrap(), fg, bg)
            );
            assert_eq!(actual.width(), 1);
        }
    }

    #[test]
    fn braille_uniform_and_same_brightness_colors_remain_opaque() {
        assert_eq!(braille([RED; 8]), styled(' ', RED, RED));
        let result = braille([RED, BLUE, RED, BLUE, RED, BLUE, RED, BLUE]);
        assert_eq!(result, styled('\u{28b8}', BLUE, RED));
        let result = braille([
            BLACK,
            WHITE,
            Rgb(10, 0, 0),
            Rgb(245, 255, 255),
            BLACK,
            WHITE,
            BLACK,
            WHITE,
        ]);
        assert_eq!(result, styled('\u{28b8}', Rgb(253, 255, 255), Rgb(3, 0, 0)));
    }

    #[test]
    fn cell_equality_tracks_presentation_and_revision() {
        let original = cell(canvas(2, 2), 2, 2);
        assert_eq!(original, original.clone());
        let mut changed = original.clone();
        changed.x = 1;
        assert_ne!(original, changed);
        changed = original.clone();
        changed.y = 1;
        assert_ne!(original, changed);
        changed = original.clone();
        changed.target.width = 3;
        assert_ne!(original, changed);
        changed = original.clone();
        changed.options.background = RED;
        assert_ne!(original, changed);
        changed = original.clone();
        changed.canvas.set_pixel(0, 0, RED);
        assert_ne!(original, changed);
    }

    #[test]
    fn cell_sampling_is_a_crop_of_the_full_target_for_every_placement() {
        let mut source = canvas(6, 3);
        for y in 0..3 {
            for x in 0..6 {
                source.set_pixel(x, y, Rgb(x as u8, y as u8, 99));
            }
        }
        let pointer = source.pixels().as_ptr();
        let version = source.version();
        for scale in [RasterScale::Fit, RasterScale::Stretch] {
            for align_x in [RasterAlign::Start, RasterAlign::Center, RasterAlign::End] {
                for align_y in [RasterAlign::Start, RasterAlign::Center, RasterAlign::End] {
                    let mut whole = cell(source.clone(), 1, 1);
                    whole.options = RasterOptions {
                        background: WHITE,
                        scale,
                        align_x,
                        align_y,
                        fallback: RasterFallback::HalfBlock,
                    };
                    let mut fragment = whole.clone();
                    fragment.target = Size::from((3, 2));
                    for y in 0..2 {
                        for x in 0..3 {
                            fragment.x = x;
                            fragment.y = y;
                            for pixel_y in 0..4 {
                                for pixel_x in 0..3 {
                                    assert_eq!(
                                        fragment.sample(pixel_x, pixel_y, 3, 4),
                                        whole.sample(
                                            u32::from(x) * 3 + pixel_x,
                                            u32::from(y) * 4 + pixel_y,
                                            9,
                                            8,
                                        )
                                    );
                                }
                            }
                            fragment.fallback();
                            fragment.options.fallback = RasterFallback::Braille;
                            fragment.fallback();
                        }
                    }
                    assert_eq!(fragment.canvas.pixels().as_ptr(), pointer);
                    assert_eq!(fragment.canvas.version(), version);
                }
            }
        }
    }

    #[test]
    fn exact_pixel_limit_is_accepted_and_sampled_without_overflow() {
        let source = RasterCanvas::new(MAX_RASTER_PIXELS as u32, 1, RED).unwrap();
        assert_eq!(source.pixels().len(), MAX_RASTER_PIXELS);
        let mut sample = cell(source, u16::MAX, u16::MAX);
        sample.options.scale = RasterScale::Stretch;
        sample.x = u16::MAX - 1;
        sample.y = u16::MAX - 1;
        assert_eq!(sample.sample(65534, 65534, u16::MAX, u16::MAX), RED);
    }
}
