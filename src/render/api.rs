mod builder;
mod draw_err;
mod draw_update;
#[path = "render.rs"]
mod renderer;
mod wake;

pub mod area;
pub mod buffer;
pub mod chunk;
pub mod event_loop;
pub mod style;
pub mod text;
pub use builder::{Builder, InlineMouseMode};
pub use draw_err::DrawErr;
pub use draw_update::*;
pub use renderer::Render;
pub use wake::WakeHandle;
