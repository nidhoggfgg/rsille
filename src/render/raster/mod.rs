//! Opaque RGB drawing and terminal presentation.
//!
//! Pixel storage and sampling are independent of the output protocol.
//! Terminal raster presentation is Sixel-only; [`output`] configures the required
//! host capabilities. The presenter and optional Sixel encoder remain renderer
//! implementation details. Raster images are never converted to characters.
//!
//! # Drawing and presentation
//!
//! - [`RasterCanvas`] owns fixed-size RGB pixels and provides clipped drawing
//!   primitives. Clones are cheap copy-on-write snapshots.
//! - [`RasterOptions`] controls scaling, alignment, and the opaque presentation
//!   background, without modifying the source pixels.
//! - [`RasterCell`] describes one terminal cell of a full image presentation.
//!   Its offsets remain relative to the original target even after clipping.
//! - [`output::RasterOutput`] describes verified terminal capabilities and
//!   physical pixels per character cell.
//!
//! Drawing, importing pixels, and sampling work without Cargo features or a
//! terminal. Native presentation requires the `sixel` feature and explicit
//! capability configuration; unknown capabilities cause an error rather than a
//! character-based fallback. Measuring cell pixels does not establish Sixel
//! support. See [`output::CellPixels::probe`] for startup probing restrictions.
//!
//! Native output uses one positioned tile per visible terminal cell, favoring
//! precise clipping and ownership over transport throughput; tiles are not merged.
//! Colors use a fixed RGB332 palette (3 red, 3 green, 2 blue bits), not lossless
//! 24-bit output or adaptive quantization. Source pixels remain unchanged.
//!
//! # Example
//!
//! Sample a two-pixel image stretched across one terminal cell, without terminal
//! I/O. For normal TUI use, prefer
//! [`RasterCanvasWidget`](crate::tui::widgets::display::raster_canvas::RasterCanvasWidget).
//!
//! ```
//! use rsille::render::raster::{
//!     RasterCanvas, RasterCell, RasterOptions, RasterScale, Rgb,
//! };
//!
//! let canvas = RasterCanvas::from_pixels(
//!     2, 1, Rgb(0, 0, 0), vec![Rgb(255, 0, 0), Rgb(0, 0, 255)],
//! )?;
//! let cell = RasterCell {
//!     canvas,
//!     target: (1, 1).into(),
//!     x: 0,
//!     y: 0,
//!     options: RasterOptions { scale: RasterScale::Stretch, ..Default::default() },
//! };
//! assert_eq!(cell.sample(0, 0, 8, 16), Rgb(255, 0, 0));
//! assert_eq!(cell.sample(7, 15, 8, 16), Rgb(0, 0, 255));
//! # Ok::<(), rsille::render::raster::RasterError>(())
//! ```

mod canvas;
pub mod output;
mod presenter;
#[cfg(feature = "sixel")]
mod sixel;

pub use canvas::*;
pub(super) use presenter::{RasterPlan, RasterPresenter};
