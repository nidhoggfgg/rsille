mod builder;
mod draw_err;
mod draw_update;
mod input;
#[path = "render.rs"]
mod renderer;
mod terminal;
mod terminal_probe;
mod wake;

pub mod area;
pub mod buffer;
pub mod chunk;
pub mod event_loop;
pub mod raster;
pub mod style;
pub mod text;
pub use builder::{Builder, InlineMouseMode};
pub use draw_err::DrawErr;
pub use draw_update::*;
pub use input::{InputCapabilities, KeyboardMode};
/// Compatibility path for raster output configuration; see [`raster::output`].
pub use raster::output as raster_output;
pub use renderer::Render;
pub use wake::WakeHandle;
