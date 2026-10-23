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
    pub fn force_follows_viewport(self) -> bool {
        matches!(self, Self::ForceFollowViewport)
    }
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct Builder {
    pub(super) enable_raw_mode: bool,
    pub(super) enable_alt_screen: bool,
    pub(super) enable_mouse_capture: bool,
    pub(super) enable_hide_cursor: bool,
    pub(super) keyboard_mode: KeyboardMode,
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
    pub fn new() -> Self {
        Self {
            size: Size::default(),
            pos: Position::default(),
            enable_raw_mode: false,
            enable_alt_screen: false,
            enable_mouse_capture: false,
            enable_hide_cursor: false,
            keyboard_mode: KeyboardMode::Legacy,
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

    /// Request terminal focus gained/lost events independently of mouse capture.
    pub fn focus_events(&mut self, enabled: bool) -> &mut Self {
        self.focus_events = enabled;
        self
    }

    pub fn enable_raw_mode(&mut self) -> &mut Self {
        self.enable_raw_mode = true;
        self
    }

    pub fn disable_raw_mode(&mut self) -> &mut Self {
        self.enable_raw_mode = false;
        self
    }

    pub fn enable_alt_screen(&mut self) -> &mut Self {
        self.enable_alt_screen = true;
        self
    }

    pub fn disable_alt_screen(&mut self) -> &mut Self {
        self.enable_alt_screen = false;
        self
    }

    pub fn enable_mouse_capture(&mut self) -> &mut Self {
        self.enable_mouse_capture = true;
        self
    }

    pub fn disable_mouse_capture(&mut self) -> &mut Self {
        self.enable_mouse_capture = false;
        self
    }

    pub fn enable_hide_cursor(&mut self) -> &mut Self {
        self.enable_hide_cursor = true;
        self
    }

    pub fn disable_hide_cursor(&mut self) -> &mut Self {
        self.enable_hide_cursor = false;
        self
    }

    pub fn max_event_per_frame(&mut self, max_event_per_frame: usize) -> &mut Self {
        self.max_event_per_frame = max_event_per_frame;
        self
    }

    pub fn frame_limit(&mut self, frame_limit: u16) -> &mut Self {
        self.frame_limit = Some(frame_limit);
        self
    }

    pub fn size(&mut self, size: (u16, u16)) -> &mut Self {
        self.size = size.into();
        self
    }

    pub fn pos(&mut self, pos: (u16, u16)) -> &mut Self {
        self.pos = pos.into();
        self
    }

    pub fn clear(&mut self, clear: bool) -> &mut Self {
        self.clear = clear;
        self
    }

    pub fn append_newline(&mut self, append_newline: bool) -> &mut Self {
        self.append_newline = append_newline;
        self
    }

    /// Enable inline mode for non-fullscreen, CLI-style interaction
    ///
    /// When enabled:
    /// - Disables alternate screen (renders to current terminal position)
    /// - Keeps cursor visible by default
    /// - Suitable for modern CLI tools (npm, yarn style)
    /// - Automatically sets clear=false to preserve previous output
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

    /// Set maximum height for inline mode (default: 50)
    ///
    /// This limits how many lines inline mode can use when rendering.
    /// Actual height will be min(content_height, max_height, terminal_height)
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

    /// Set the exit key code (default: ESC)
    ///
    /// When the user presses this key, the event loop will automatically exit.
    /// Use `disable_exit_code()` if you want to handle exit logic in your application.
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
