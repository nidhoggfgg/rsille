//! Terminal applications, Unicode text rendering, and sample-based graphics.
//!
//! # Installation
//!
//! ```toml
//! [dependencies]
//! rsille = "3"
//! ```
//!
//! No optional features are enabled by default. Enable `syntax-highlighting` for
//! Syntect highlighting in code viewers; plain text and navigation work without it.
//!
//! # Choose an API
//!
//! | Module | Use it for | Starting points |
//! | --- | --- | --- |
//! | [`tui`] | Stateful applications with widgets, layout, and input | [`App`](tui::app::App), [`prelude`](tui::prelude) |
//! | [`render`] | Terminal buffers, styled text, and custom rendering loops | [`Buffer`](render::buffer::Buffer), [`Builder`](render::Builder), [`Draw`](render::Draw) |
//! | [`canvas`] | Drawing graphics with Braille, half blocks, or quadrants | [`Canvas`](canvas::Canvas), [`Encoding`](canvas::Encoding), [`Paint`](canvas::Paint) |
//!
//! # Minimal application
//!
//! ```no_run
//! use rsille::tui::prelude::*;
//!
//! fn main() -> WidgetResult<()> {
//!     App::new(0_u32)
//!         .mode(Mode::Inline)
//!         .update(|count, (), _ctx| *count += 1)
//!         .view(|count| {
//!             button(format!("Count: {count}")).on_click(|| ())
//!         })
//!         .run()?;
//!     Ok(())
//! }
//! ```
//!
//! [`App`](tui::app::App) owns the initial state and configuration. The update
//! callback handles messages and changes state; the view callback describes the
//! widget tree. Configure services, themes, and input before calling
//! [`App::update`](tui::app::App::update).
//!
//! Tab moves focus, Enter or Space activates the focused button, and Esc exits
//! by default. [`Mode::Inline`](tui::app::Mode::Inline) uses the normal terminal
//! screen; the default mode is fullscreen.
//!
//! [`Application::run`](tui::app::Application::run) returns the final state and
//! exit reason after terminal cleanup. Inside an existing Tokio runtime with the
//! time driver enabled, use [`run_async`](tui::app::Application::run_async).
//!
//! # API reference by task
//!
//! - **Widgets and layout:** [`tui::widgets`], [`tui::layout`], and [`tui::style`].
//! - **Input and focus:** [`tui::event`], [`tui::focus`], and [`tui::gesture`].
//! - **Background work:** [`UpdateCtx`](tui::effect::UpdateCtx), [`tui::effect`],
//!   and [`tui::subscription`].
//! - **Animation and visual effects:** [`tui::animation`] and
//!   [`visual`](tui::widgets::visual()).
//! - **Custom widgets:** [`Widget`](tui::widget::Widget) and [`tui::widget`].
//! - **Text and terminal cells:** [`tui::text`], [`render::text`], and
//!   [`Buffer`](render::buffer::Buffer).
//! - **Graphics in a layout:** [`canvas`](tui::widgets::canvas()) and
//!   [`CanvasWidget`](tui::widgets::CanvasWidget).
//!
//! Canvas drawing coordinates are samples, with the y-axis increasing upward.
//! Terminal buffer coordinates and layout sizes use columns and rows. See
//! [`Encoding`](canvas::Encoding) for the number of samples per terminal cell.
//!
//! # Testing without a terminal
//!
//! Use [`Application::build`](tui::app::Application::build) to create an
//! in-memory [`Runtime`](tui::runtime::Runtime), send messages, and advance it
//! explicitly:
//!
//! ```
//! use rsille::tui::prelude::*;
//!
//! let mut runtime = App::new(0_u32)
//!     .update(|count, (), _ctx| *count += 1)
//!     .view(|count| button(format!("Count: {count}")).on_click(|| ()))
//!     .build()
//!     .size(40, 3);
//!
//! runtime.handle().send(()).unwrap();
//! runtime.step(&[]).unwrap();
//! assert_eq!(*runtime.state(), 1);
//! assert_eq!(runtime.frame().size().width, 40);
//! ```
//!
//! [`ManualClock`](tui::runtime::ManualClock) and
//! [`ManualExecutor`](tui::runtime::ManualExecutor) provide control over timers
//! and queued work in tests.

/// Terminal buffers, text layout, styled drawing, and event-loop integration.
///
/// Use [`Buffer`](render::buffer::Buffer) for in-memory terminal cells and
/// [`Chunk`](render::chunk::Chunk) for bounded drawing with a local origin.
/// [`Draw`](render::Draw) connects custom content to the renderer. Terminal I/O
/// is provided by [`Render`](render::Render) and
/// [`EventLoop`](render::event_loop::EventLoop), configured through
/// [`Builder`](render::Builder).
///
/// Coordinates and dimensions are terminal columns and rows. The
/// [`text`](render::text) module defines grapheme, display-width, and tab policies;
/// [`style`](render::style) provides terminal colors and attributes.
///
/// # Examples
///
/// ```
/// use rsille::render::{area::Size, buffer::Buffer, chunk::Chunk, style::Style};
///
/// let mut buffer = Buffer::new((12, 2).into());
/// {
///     let mut chunk = Chunk::new(&mut buffer, Size::from((12, 2)).into())?;
///     chunk.set_string(0, 0, "Hello", Style::default())?;
/// }
/// assert_eq!(buffer.size(), Size::from((12, 2)));
/// # Ok::<(), rsille::render::DrawErr>(())
/// ```
#[path = "render/api.rs"]
pub mod render;

/// Independent opaque RGB drawing, shared with the renderer.
///
/// Unlike [`canvas`], raster coordinates are explicit pixels with y increasing
/// downward. Image presentation requires Sixel; headless frames retain raster metadata
/// and opaque background placeholders, not character previews.
pub use render::raster;

/// Sample-based graphics with Braille, half-block, and quadrant terminal encoding.
///
/// [`Canvas`](canvas::Canvas) stores colored samples and encodes them into cells
/// when drawn or printed. [`Encoding`](canvas::Encoding) selects the samples per
/// cell; sample coordinates grow rightward and upward, while output bounds use
/// terminal cells. Implement [`Paint`](canvas::Paint) to reuse drawing routines.
/// For graphics embedded in a widget tree, use
/// [`CanvasWidget`](tui::widgets::CanvasWidget).
///
/// # Examples
///
/// ```
/// use rsille::canvas::{Canvas, Encoding};
///
/// let mut canvas = Canvas::with_encoding(Encoding::HalfBlock);
/// canvas.line((0, 0), (4, 4));
/// assert_eq!(canvas.encoding().samples_per_cell(), (1, 2));
/// let mut output = Vec::new();
/// canvas.print_on(&mut output, false)?;
/// assert!(!output.is_empty());
/// # Ok::<(), std::io::Error>(())
/// ```
#[path = "canvas/api.rs"]
pub mod canvas;

/// Stateful terminal applications with widgets, layout, effects, and animation.
///
/// Start with [`App`](tui::app::App) or import [`prelude`](tui::prelude).
/// The update callback changes application state and schedules work; the view
/// builds a widget description from that state. A
/// [`Runtime`](tui::runtime::Runtime) can run in a terminal or be stepped by tests
/// and embedding hosts without a TTY.
///
/// - [`widgets`](tui::widgets), [`layout`](tui::layout), and [`style`](tui::style)
///   provide built-in UI composition and presentation.
/// - [`widget`](tui::widget) defines custom widget, lifecycle, and state APIs.
/// - [`event`](tui::event), [`focus`](tui::focus), and [`gesture`](tui::gesture)
///   handle input routing, keyboard focus, and pointer interaction.
/// - [`effect`](tui::effect), [`subscription`](tui::subscription), and
///   [`scope`](tui::scope) manage background work and component resources.
/// - [`animation`](tui::animation) supplies timing and transition policies.
/// - [`state`](tui::state) and [`shell`](tui::shell) hold application helper state.
///
/// Enable the `syntax-highlighting` feature for syntect-backed code highlighting.
#[path = "tui/api.rs"]
pub mod tui;
