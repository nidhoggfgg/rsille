//! Opaque RGB drawing and terminal presentation.
//!
//! Pixel storage and sampling are independent of the output protocol.
//! Terminal raster presentation is Sixel-only; [`output`] configures the required
//! host capabilities. The presenter and optional Sixel encoder remain renderer
//! implementation details. Raster images are never converted to characters.

mod canvas;
pub mod output;
mod presenter;
#[cfg(feature = "sixel")]
mod sixel;

pub use canvas::*;
pub(super) use presenter::{RasterPlan, RasterPresenter};
