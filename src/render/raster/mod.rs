//! Opaque RGB drawing and terminal presentation.
//!
//! Pixel storage, sampling, and character fallback are independent of the output
//! protocol. [`output`] selects character fallback or native Sixel presentation;
//! the presenter and optional Sixel encoder remain renderer implementation details.

mod canvas;
pub mod output;
mod presenter;
#[cfg(feature = "sixel")]
mod sixel;

pub use canvas::*;
pub(super) use presenter::{RasterPlan, RasterPresenter};
