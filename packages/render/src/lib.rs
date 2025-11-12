mod builder;
mod draw_err;
mod draw_update;
mod render;

pub mod area;
pub mod buffer;
pub mod chunk;
pub mod event_loop;
pub mod style;

/// Wrapper around crossterm's `queue!` macro with debug logging support.
///
/// In debug builds, this macro logs all terminal operations at trace level,
/// which is helpful for debugging rendering issues. In release builds,
/// it has zero runtime overhead as the logging is completely removed.
///
/// # Example
/// ```ignore
/// queue_with_log!(stdout, MoveTo(5, 10), Print("Hello"))?;
/// ```
#[macro_export]
macro_rules! queue_with_log {
    ($dst:expr, $($cmd:expr),* $(,)?) => {{
        #[cfg(debug_assertions)]
        {
            $(
                log::trace!(target: "render::queue", "{}", stringify!($cmd));
            )*
        }
        crossterm::queue!($dst, $($cmd),*)
    }};
}

pub use builder::Builder;
pub use draw_err::DrawErr;
pub use draw_update::*;
pub use render::Render;
