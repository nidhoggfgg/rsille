//! RGB drawing with optional pixel transparency and terminal presentation.
//!
//! Pixel storage and sampling are independent of the output protocol.
//! Use [`output::RasterBackend`] for startup selection or [`output::RasterSession`]
//! for host-verified fullscreen or inline output. Inline Sixel also accepts [`output::RasterOutput`].
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
//! terminal. Managed fullscreen output negotiates only when requested. On Unix
//! (including WSL), Auto prefers the compiled Kitty backend, then Sixel.
//! Kitty uses direct RGBA uploads and clipped placements. Reusing a displayed
//! surface avoids another upload until it is invalidated or removed. Terminal
//! eviction is silent. Hosts can explicitly release resources to force a refresh.
//! Kitty limits visible layer cells (and thus placements) to 32,768, and visible
//! physical pixels to 16,777,216 per frame. Resident RGBA data and staged protocol
//! bytes each have a 32 MiB limit. Large targets are sampled in visible blocks.
//!
//! Sixel uses one positioned tile per visible terminal cell.
//! Colors use a fixed RGB332 palette (3 red, 3 green, 2 blue bits), not lossless
//! 24-bit output or adaptive quantization. Transparent pixels leave existing
//! terminal content untouched; they do not erase previously drawn graphics. Source pixels remain unchanged.
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
#[cfg(feature = "kitty")]
mod kitty;
pub mod output;
mod presenter;
#[cfg(feature = "sixel")]
mod sixel;
mod sixel_presenter;

pub use canvas::*;
pub use output::{RasterBackend, RasterSession};
pub(super) use presenter::{RasterPlan, RasterPresenter};
