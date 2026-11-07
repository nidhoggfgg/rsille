#![doc = include_str!("../README.md")]

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
