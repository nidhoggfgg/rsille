//! Render the **inside** of a Typst math equation into an opaque RGB canvas.
//!
//! No LaTeX conversion, filesystem discovery, package downloads or system fonts
//! are involved. Supply OpenType/TrueType font bytes to [`MathRenderer::new`].
//! A font with an OpenType MATH table is required for the chosen family.
//!
//! ```no_run
//! use rsille_typst::{MathRenderer, RenderOptions};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let bytes = std::fs::read("NewCMMath-Regular.otf")?;
//! let renderer = MathRenderer::new([bytes], "New Computer Modern Math")?;
//! let canvas = renderer.render(r"sum_(k=1)^n k = (n(n+1))/2", &RenderOptions::default())?;
//! assert!(!canvas.pixels().is_empty());
//! # Ok(()) }
//! ```
//!
//! File access restrictions are **not a sandbox**: arbitrary Typst expressions
//! can exhaust CPU or memory before raster size checks. Isolate untrusted input
//! in a resource-limited worker process. See the package README for limitations.

use std::fmt;

use rsille::raster::MAX_RASTER_PIXELS;
pub use rsille::raster::{RasterCanvas, RasterError, Rgb};
pub use typst::diag::SourceDiagnostic;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, Value};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

const TEMPLATE: &str = r#"#set page(width: auto, height: auto, margin: 0pt, fill: none)
#set text(font: sys.inputs.family, size: sys.inputs.size * 1pt, fill: rgb(sys.inputs.foreground))
#math.equation(block: true, eval(sys.inputs.formula, mode: "math"))"#;

/// Options for a single rendering. Colors are opaque sRGB values.
#[derive(Clone, Debug)]
pub struct RenderOptions {
    /// Font size in points (1/72 inch), finite and in `0.1..=4096`. Default: 18.
    pub font_size: f64,
    /// Pixels per typographic point, finite and in `0.01..=16`. Default: 2.
    pub pixels_per_point: f32,
    /// Default text color. Typst expressions may override it. Default: black.
    pub foreground: Rgb,
    /// Opaque background used for alpha compositing and canvas metadata.
    /// Default: white.
    pub background: Rgb,
    /// Maximum output pixels, in `1..=rsille::raster::MAX_RASTER_PIXELS`.
    /// Default: the core limit (16 Mi pixels). Each dimension is also limited
    /// to 16384 pixels. These limits do not bound compilation memory or time.
    pub max_pixels: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            font_size: 18.0,
            pixels_per_point: 2.0,
            foreground: Rgb(0, 0, 0),
            background: Rgb(255, 255, 255),
            max_pixels: MAX_RASTER_PIXELS,
        }
    }
}

impl RenderOptions {
    fn validate(&self) -> Result<(), Error> {
        if !self.font_size.is_finite() || !(0.1..=4096.0).contains(&self.font_size) {
            return Err(Error::InvalidOptions(
                "font_size must be finite and in 0.1..=4096",
            ));
        }
        if !self.pixels_per_point.is_finite() || !(0.01..=16.0).contains(&self.pixels_per_point) {
            return Err(Error::InvalidOptions(
                "pixels_per_point must be finite and in 0.01..=16",
            ));
        }
        if self.max_pixels == 0 || self.max_pixels > MAX_RASTER_PIXELS {
            return Err(Error::InvalidOptions(
                "max_pixels must be within the core raster limit",
            ));
        }
        Ok(())
    }
}

/// Failure to load fonts, compile math, or construct the raster output.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A font input contained no readable font faces (zero-based input index).
    InvalidFont(usize),
    /// The requested family is absent or has no OpenType MATH table.
    MissingMathFont(String),
    /// An option failed validation before compilation.
    InvalidOptions(&'static str),
    /// Original Typst errors, including messages, hints, spans and traces.
    /// `eval` diagnostics refer to the template call, not formula byte offsets.
    Compile(Vec<SourceDiagnostic>),
    /// The result was not exactly one page.
    PageCount(usize),
    /// Non-finite, negative or oversized page dimensions, or too many pixels.
    RasterLimit,
    /// A failure reported by the core raster resource or RGB allocation.
    Raster(RasterError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFont(index) => write!(f, "font input {index} contains no readable faces"),
            Self::MissingMathFont(family) => {
                write!(f, "no OpenType math font in family {family:?}")
            }
            Self::InvalidOptions(message) => f.write_str(message),
            Self::Compile(errors) => {
                f.write_str("Typst compilation failed")?;
                for error in errors {
                    write!(f, "\n{}", error.message)?;
                    for hint in &error.hints {
                        write!(f, "\n  hint: {hint}")?;
                    }
                }
                Ok(())
            }
            Self::PageCount(count) => write!(f, "expected one formula page, got {count}"),
            Self::RasterLimit => f.write_str("formula exceeds raster dimensions or pixel limit"),
            Self::Raster(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Raster(error) => Some(error),
            _ => None,
        }
    }
}

impl From<RasterError> for Error {
    fn from(error: RasterError) -> Self {
        Self::Raster(error)
    }
}

/// Successful rendering with nonfatal Typst diagnostics retained.
#[derive(Debug)]
pub struct RenderedMath {
    /// Opaque RGB pixels, ready for `raster_canvas` or `Chunk::raster`.
    pub canvas: RasterCanvas,
    /// Original warnings returned by Typst compilation/layout.
    /// Typst 0.13.1 discards warnings produced inside immediate `eval` calls;
    /// those warnings cannot be recovered through this interface.
    pub warnings: Vec<SourceDiagnostic>,
}

/// Reusable font resources for isolated, single-formula compilations.
///
/// Font bytes are copied once and parsed at construction. All faces in font
/// collections are registered in input order. No formula-result cache is kept
/// by this crate; Typst itself uses process-global memoization/interner storage.
/// Creating or dropping this object is not a memory isolation boundary.
pub struct MathRenderer {
    fonts: Vec<Font>,
    book: LazyHash<FontBook>,
    family: String,
    source: Source,
}

impl MathRenderer {
    /// Load explicitly provided TTF/OTF/TTC bytes and select a math font family.
    ///
    /// There are no implicit or bundled fonts in the public library. Each input
    /// must contain at least one readable face, and the requested family must
    /// contain a face with a MATH table. Font licensing remains the caller's
    /// responsibility. Additional fonts can supply text/glyph fallbacks.
    pub fn new<I, B>(font_bytes: I, math_family: impl Into<String>) -> Result<Self, Error>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut fonts = Vec::new();
        for (index, bytes) in font_bytes.into_iter().enumerate() {
            let start = fonts.len();
            fonts.extend(Font::iter(Bytes::new(bytes.as_ref().to_vec())));
            if fonts.len() == start {
                return Err(Error::InvalidFont(index));
            }
        }
        let family = math_family.into();
        let book = FontBook::from_fonts(&fonts);
        if !book
            .select_family(&family.to_lowercase())
            .any(|index| fonts[index].ttf().tables().math.is_some())
        {
            return Err(Error::MissingMathFont(family));
        }
        Ok(Self {
            fonts,
            book: LazyHash::new(book),
            family,
            source: Source::detached(TEMPLATE),
        })
    }

    /// Render math-mode contents, without surrounding `$` delimiters.
    ///
    /// For example: `r"frac(1, 2) + sqrt(x)"`. This is Typst syntax, not LaTeX.
    /// Use [`Self::render_with_diagnostics`] to retain nonfatal warnings.
    pub fn render(&self, formula: &str, options: &RenderOptions) -> Result<RasterCanvas, Error> {
        Ok(self.render_with_diagnostics(formula, options)?.canvas)
    }

    /// Render one formula and retain nonfatal diagnostics.
    ///
    /// The controlled template has automatic dimensions and zero page margins.
    /// Formula text is passed as data and evaluated only in math mode. Hash
    /// expressions remain supported, but all external file/package requests
    /// are denied. Compilation is synchronous and has no time/memory budget.
    pub fn render_with_diagnostics(
        &self,
        formula: &str,
        options: &RenderOptions,
    ) -> Result<RenderedMath, Error> {
        options.validate()?;
        let Rgb(r, g, b) = options.foreground;
        let mut inputs = Dict::new();
        inputs.insert("family".into(), Value::Str(self.family.as_str().into()));
        inputs.insert("formula".into(), Value::Str(formula.into()));
        inputs.insert("size".into(), Value::Float(options.font_size));
        inputs.insert(
            "foreground".into(),
            Value::Str(format!("#{r:02x}{g:02x}{b:02x}").into()),
        );
        let world = FormulaWorld {
            renderer: self,
            library: LazyHash::new(Library::builder().with_inputs(inputs).build()),
        };
        let result = typst::compile::<PagedDocument>(&world);
        let document = result
            .output
            .map_err(|errors| Error::Compile(errors.into_iter().collect()))?;
        if document.pages.len() != 1 {
            return Err(Error::PageCount(document.pages.len()));
        }
        let page = &document.pages[0];
        let size = page.frame.size();
        let (width, height) = checked_dimensions(size.x.to_pt(), size.y.to_pt(), options)?;
        let pixmap = typst_render::render(page, options.pixels_per_point);
        // Keep the preflight coupled to the pinned renderer's rounding rules.
        if pixmap.width() != width || pixmap.height() != height {
            return Err(Error::RasterLimit);
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(width as usize * height as usize)
            .map_err(|_| RasterError::AllocationFailed)?;
        pixels.extend(pixmap.pixels().iter().map(|pixel| {
            composite(
                [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()],
                options.background,
            )
        }));
        Ok(RenderedMath {
            canvas: RasterCanvas::from_pixels(width, height, options.background, pixels)?,
            warnings: result.warnings.into_iter().collect(),
        })
    }
}

fn checked_dimensions(
    width: f64,
    height: f64,
    options: &RenderOptions,
) -> Result<(u32, u32), Error> {
    let dimension = |points: f64| {
        if !points.is_finite() || points < 0.0 {
            return Err(Error::RasterLimit);
        }
        // typst-render 0.13.1 multiplies in f32, rounds, then clamps to one.
        let pixels = (options.pixels_per_point * points as f32).round().max(1.0);
        if !pixels.is_finite() || pixels > 16384.0 {
            return Err(Error::RasterLimit);
        }
        Ok(pixels as u32)
    };
    let (width, height) = (dimension(width)?, dimension(height)?);
    let count = u64::from(width) * u64::from(height);
    if count > options.max_pixels as u64 || count > MAX_RASTER_PIXELS as u64 {
        return Err(Error::RasterLimit);
    }
    Ok((width, height))
}

fn composite([r, g, b, a]: [u8; 4], background: Rgb) -> Rgb {
    // tiny-skia stores premultiplied sRGB bytes: do not multiply RGB by alpha
    // again (or unpremultiply first). Round the background contribution once.
    let channel = |value: u8, bg: u8| {
        (u32::from(value) + (u32::from(bg) * (255 - u32::from(a)) + 127) / 255).min(255) as u8
    };
    Rgb(
        channel(r, background.0),
        channel(g, background.1),
        channel(b, background.2),
    )
}

struct FormulaWorld<'a> {
    renderer: &'a MathRenderer,
    library: LazyHash<Library>,
}

impl World for FormulaWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &self.renderer.book
    }
    fn main(&self) -> FileId {
        self.renderer.source.id()
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main() {
            Ok(self.renderer.source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }
    fn file(&self, _: FileId) -> FileResult<Bytes> {
        Err(FileError::AccessDenied)
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.renderer.fonts.get(index).cloned()
    }
    fn today(&self, _: Option<i64>) -> Option<Datetime> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_only_exposes_main_source_and_registered_fonts() {
        use typst::syntax::VirtualPath;

        let renderer =
            MathRenderer::new(typst_assets::fonts(), "New Computer Modern Math").unwrap();
        let world = FormulaWorld {
            renderer: &renderer,
            library: LazyHash::new(Library::default()),
        };
        assert!(world.source(world.main()).is_ok());
        assert!(matches!(
            world.file(world.main()),
            Err(FileError::AccessDenied)
        ));
        for package in [None, Some("@preview/cetz:0.3.4".parse().unwrap())] {
            let id = FileId::new(package, VirtualPath::new("/outside.typ"));
            assert!(matches!(world.source(id), Err(FileError::AccessDenied)));
            assert!(matches!(world.file(id), Err(FileError::AccessDenied)));
        }
        assert!(world.font(0).is_some());
        assert!(world.font(usize::MAX).is_none());
        assert!(world.today(None).is_none());
        assert!(world.today(Some(0)).is_none());
    }

    #[test]
    fn premultiplied_compositing() {
        let bg = Rgb(20, 40, 60);
        assert_eq!(composite([0, 0, 0, 0], bg), bg);
        assert_eq!(composite([100, 50, 25, 255], bg), Rgb(100, 50, 25));
        assert_eq!(composite([100, 50, 25, 128], bg), Rgb(110, 70, 55));
    }

    #[test]
    fn preflight_rounding_and_limits() {
        let options = RenderOptions::default();
        assert_eq!(checked_dimensions(0.0, 0.0, &options).unwrap(), (1, 1));
        assert_eq!(checked_dimensions(1.25, 2.24, &options).unwrap(), (3, 4));
        for value in [f64::NAN, f64::INFINITY, -1.0, f64::MAX, 8192.5] {
            assert!(checked_dimensions(value, 1.0, &options).is_err());
        }
        assert!(checked_dimensions(4096.0, 4096.0, &options).is_err());
    }
}
