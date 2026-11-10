//! Configuration shared by standalone renderers and terminal event loops.

use crossterm::event::{KeyCode, KeyEvent};
use log::info;

use crate::render::{
    area::{Position, Size},
    event_loop::EventLoop,
    Draw, DrawUpdate, KeyboardMode, Render,
};

/// Mouse handling policy for inline rendering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum InlineMouseMode {
    /// Translate terminal mouse coordinates into the inline UI's local space.
    #[default]
    Local,
    /// Aggressively keep the terminal viewport at the live output position.
    ///
    /// This emits a newline after each inline frame and moves the cursor back to
    /// the rendered UI, relying on common terminal scroll-on-output behavior.
    ForceFollowViewport,
}

impl InlineMouseMode {
    /// Returns whether each inline frame should force the viewport to follow.
    pub fn force_follows_viewport(self) -> bool {
        matches!(self, Self::ForceFollowViewport)
    }
}

/// Configures a [`Render`] or [`EventLoop`] without touching the terminal.
///
/// Set [`size`](Self::size) before building: the default buffer is empty.
/// Terminal setup options apply to event loops; standalone renderers only
/// draw and write output. Setters mutate this builder and can be chained.
///
/// ```
/// use rsille::render::Builder;
///
/// let mut builder = Builder::new();
/// builder.size((80, 24)).enable_all().frame_limit(60);
/// ```
#[derive(Debug, Clone, Copy, Hash)]
pub struct Builder {
    pub(super) enable_raw_mode: bool,
    pub(super) enable_alt_screen: bool,
    pub(super) enable_mouse_capture: bool,
    pub(super) enable_hide_cursor: bool,
    pub(super) keyboard_mode: KeyboardMode,
    pub(super) raster_output: crate::render::raster_output::RasterOutput,
    pub(super) focus_events: bool,
    pub(super) exit_code: Option<KeyEvent>,
    pub(super) frame_limit: Option<u16>,
    pub(super) max_event_per_frame: usize,
    pub(super) size: Size,
    pub(super) pos: Position,
    pub(super) clear: bool,
    pub(super) append_newline: bool,
    pub(super) inline_mode: bool,
    pub(super) inline_max_height: u16,
    pub(super) inline_mouse_mode: InlineMouseMode,
}

impl Builder {
    /// Creates a builder with a zero-sized buffer at `(0, 0)`.
    ///
    /// Raw mode, alternate screen, mouse capture, cursor hiding, focus events,
    /// and inline mode are disabled. Keyboard input uses the legacy protocol.
    /// The exit key is Escape, the event batch limit is 10, and no periodic
    /// frame interval is configured. The first render clears the screen and
    /// frames do not append a newline. Inline height is capped at 50 rows.
    pub fn new() -> Self {
        Self {
            size: Size::default(),
            pos: Position::default(),
            enable_raw_mode: false,
            enable_alt_screen: false,
            enable_mouse_capture: false,
            enable_hide_cursor: false,
            keyboard_mode: KeyboardMode::Legacy,
            raster_output: crate::render::raster_output::RasterOutput::default(),
            focus_events: false,
            exit_code: Some(KeyCode::Esc.into()),
            frame_limit: None,
            max_event_per_frame: 10,
            clear: true,
            append_newline: false,
            inline_mode: false,
            inline_max_height: 50,
            inline_mouse_mode: InlineMouseMode::default(),
        }
    }

    /// Enables raw mode, alternate screen, mouse capture, and cursor hiding.
    ///
    /// Keyboard negotiation and focus events retain their current settings.
    pub fn enable_all(&mut self) -> &mut Self {
        self.enable_raw_mode();
        self.enable_alt_screen();
        self.enable_mouse_capture();
        self.enable_hide_cursor();
        self
    }

    /// Select keyboard negotiation. `Auto` requires raw mode and may fall back.
    pub fn keyboard_mode(&mut self, mode: KeyboardMode) -> &mut Self {
        self.keyboard_mode = mode;
        self
    }

    /// Configures raster presentation. Unknown capabilities default to text.
    ///
    /// A standalone renderer does not enter an alternate screen: when enabling
    /// native output, the caller must establish the configured terminal modes.
    pub fn raster_output(
        &mut self,
        config: crate::render::raster_output::RasterOutput,
    ) -> &mut Self {
        self.raster_output = config;
        self
    }

    /// Request terminal focus gained/lost events independently of mouse capture.
    pub fn focus_events(&mut self, enabled: bool) -> &mut Self {
        self.focus_events = enabled;
        self
    }

    /// Enables raw input and bracketed paste during event-loop execution.
    pub fn enable_raw_mode(&mut self) -> &mut Self {
        self.enable_raw_mode = true;
        self
    }

    /// Leaves raw mode and bracketed paste setup disabled for the event loop.
    pub fn disable_raw_mode(&mut self) -> &mut Self {
        self.enable_raw_mode = false;
        self
    }

    /// Enters and clears the alternate screen when the event loop starts.
    pub fn enable_alt_screen(&mut self) -> &mut Self {
        self.enable_alt_screen = true;
        self
    }

    /// Keeps the event loop on the current terminal screen.
    pub fn disable_alt_screen(&mut self) -> &mut Self {
        self.enable_alt_screen = false;
        self
    }

    /// Requests terminal mouse events while the event loop is running.
    pub fn enable_mouse_capture(&mut self) -> &mut Self {
        self.enable_mouse_capture = true;
        self
    }

    /// Disables the event loop's request for terminal mouse events.
    pub fn disable_mouse_capture(&mut self) -> &mut Self {
        self.enable_mouse_capture = false;
        self
    }

    /// Hides the cursor during event-loop execution and restores it on cleanup.
    pub fn enable_hide_cursor(&mut self) -> &mut Self {
        self.enable_hide_cursor = true;
        self
    }

    /// Leaves cursor visibility unchanged during event-loop setup.
    pub fn disable_hide_cursor(&mut self) -> &mut Self {
        self.enable_hide_cursor = false;
        self
    }

    /// Sets the batch limit used when draining queued input events.
    ///
    /// The default is 10 events per iteration. Zero disables draining but still
    /// allows the event that woke the loop to be dispatched.
    pub fn max_event_per_frame(&mut self, max_event_per_frame: usize) -> &mut Self {
        self.max_event_per_frame = max_event_per_frame;
        self
    }

    /// Sets the periodic update rate in frames per second.
    ///
    /// Zero disables periodic polling. Input can trigger earlier iterations;
    /// this is not a strict cap on redraws. Applications providing a
    /// [`crate::render::Update::wake_handle`] use their own update deadlines.
    pub fn frame_limit(&mut self, frame_limit: u16) -> &mut Self {
        self.frame_limit = Some(frame_limit);
        self
    }

    /// Sets the initial buffer `(width, height)` in terminal cells.
    pub fn size(&mut self, size: (u16, u16)) -> &mut Self {
        self.size = size.into();
        self
    }

    /// Sets the zero-based output `(column, row)` for absolute rendering.
    ///
    /// Inline event loops determine their origin from the terminal cursor.
    pub fn pos(&mut self, pos: (u16, u16)) -> &mut Self {
        self.pos = pos.into();
        self
    }

    /// Controls clearing the entire screen before a render without a prior frame.
    ///
    /// Defaults to `true`. Alternate-screen setup clears independently.
    pub fn clear(&mut self, clear: bool) -> &mut Self {
        self.clear = clear;
        self
    }

    /// Controls appending a newline after each rendered frame; defaults to `false`.
    pub fn append_newline(&mut self, append_newline: bool) -> &mut Self {
        self.append_newline = append_newline;
        self
    }

    /// Enables rendering in the current terminal output using relative movement.
    ///
    /// Enabling this also disables alternate-screen setup and screen clearing.
    /// It does not change cursor visibility. Disabling inline mode does not
    /// restore earlier settings; later setters can override these choices.
    pub fn inline_mode(&mut self, enable: bool) -> &mut Self {
        self.inline_mode = enable;
        if enable {
            // In inline mode, we should not use alt screen
            self.enable_alt_screen = false;
            // Typically don't clear in inline mode
            self.clear = false;
        }
        self
    }

    /// Sets the inline buffer height cap used on terminal resize (default: 50).
    ///
    /// Resizes also respect the terminal height, reserving a row when forced
    /// viewport following is enabled. The initial allocation comes from
    /// [`size`](Self::size); this setting does not resize it when building.
    pub fn inline_max_height(&mut self, max_height: u16) -> &mut Self {
        self.inline_max_height = max_height;
        self
    }

    /// Configure mouse handling behavior in inline mode.
    pub fn inline_mouse_mode(&mut self, mode: InlineMouseMode) -> &mut Self {
        self.inline_mouse_mode = mode;
        self
    }

    /// Enable aggressive viewport following for inline mouse interactions.
    pub fn force_inline_mouse_follow(&mut self) -> &mut Self {
        self.inline_mouse_mode = InlineMouseMode::ForceFollowViewport;
        self
    }

    /// Sets the automatic exit key (default: Escape).
    ///
    /// Matches the code and modifiers on key press, ignoring repeats, releases,
    /// and keyboard state bits. The exit press is not sent to the application.
    /// Use [`disable_exit_code`](Self::disable_exit_code) for application-defined
    /// exit handling.
    pub fn exit_code(&mut self, key: KeyEvent) -> &mut Self {
        self.exit_code = Some(key);
        self
    }

    /// Disable automatic exit on key press
    ///
    /// When disabled, the event loop will not automatically exit on any key press.
    /// The application must handle its own exit logic.
    /// This is useful when you want full control over exit behavior, such as:
    /// - Handling ESC in dialogs/modals
    /// - Implementing custom quit confirmations
    /// - Using different exit shortcuts
    pub fn disable_exit_code(&mut self) -> &mut Self {
        self.exit_code = None;
        self
    }

    /// Builds a renderer that writes terminal commands to `writer`.
    ///
    /// Does not initialize terminal modes or draw a frame. Call
    /// [`Render::render`] to paint and flush output.
    pub fn build_render<T, W>(&self, thing: T, writer: W) -> Render<W, T>
    where
        T: Draw,
        W: std::io::Write,
    {
        info!(
            target: "render::builder",
            "building render: size={}x{}, pos=({},{}), inline={}, clear={}, alt_screen={}",
            self.size.width, self.size.height,
            self.pos.x, self.pos.y,
            self.inline_mode,
            self.clear,
            self.enable_alt_screen
        );
        Render::from_builder(self, thing, writer)
    }

    /// Builds an event loop owning `thing` and writing to standard output.
    ///
    /// Terminal setup is deferred until the loop is run.
    pub fn build_event_loop<T>(&self, thing: T) -> EventLoop<T>
    where
        T: DrawUpdate,
    {
        info!(
            target: "render::builder",
            "building event loop: fps={:?}, max_events={}, raw_mode={}, hide_cursor={}",
            self.frame_limit,
            self.max_event_per_frame,
            self.enable_raw_mode,
            self.enable_hide_cursor
        );
        EventLoop::from_builder(self, thing)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
