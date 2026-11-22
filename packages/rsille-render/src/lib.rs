//! [`buffer::Buffer`] stores terminal cells, and [`chunk::Chunk`] exposes bounded
//! drawing regions. [`Builder`] configures standalone [`Render`] instances and
//! [`event_loop::EventLoop`] execution. See [`text`] for Unicode width policies
//! and [`style`] for colors, attributes, and styled content.

mod builder;
mod draw_err;
mod draw_update;
mod input;
#[path = "render.rs"]
mod renderer;
mod terminal;
#[cfg(feature = "raster")]
mod terminal_probe;
mod wake;

pub mod area;
pub mod buffer;
pub mod chunk;
pub mod event_loop;
#[cfg(feature = "raster")]
pub mod raster;
pub mod style;
pub mod text;
pub use builder::{Builder, InlineMouseMode};
pub use draw_err::DrawErr;
pub use draw_update::*;
pub use input::{InputCapabilities, KeyboardMode};
/// Compatibility path for raster output configuration; see [`raster::output`].
#[cfg(feature = "raster")]
pub use raster::output as raster_output;
#[cfg(feature = "raster")]
pub use raster_output::{InlineRasterSession, RasterBackend, RasterSession};
pub use renderer::Render;
pub use wake::WakeHandle;
