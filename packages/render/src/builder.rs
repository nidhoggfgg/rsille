use term::crossterm::event::{KeyCode, KeyEvent};

use crate::{
    Draw, DrawUpdate, Render,
    chunk::{Position, Size},
    event_loop::EventLoop,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct Builder {
    pub(super) enable_raw_mode: bool,
    pub(super) enable_alt_screen: bool,
    pub(super) enable_mouse_capture: bool,
    pub(super) enable_hide_cursor: bool,
    pub(super) exit_code: KeyEvent,
    pub(super) frame_limit: Option<u16>,
    pub(super) max_event_per_frame: usize,
    pub(super) size: Size,
    pub(super) pos: Position,
    pub(super) clear: bool,
    pub(super) append_newline: bool,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            size: Size {
                width: 0,
                height: 0,
            },
            pos: Position { x: 0, y: 0 },
            enable_raw_mode: false,
            enable_alt_screen: false,
            enable_mouse_capture: false,
            enable_hide_cursor: false,
            exit_code: KeyCode::Esc.into(),
            frame_limit: None,
            max_event_per_frame: 10,
            clear: true,
            append_newline: false,
        }
    }

    pub fn enable_all(&mut self) -> &mut Self {
        self.enable_raw_mode();
        self.enable_alt_screen();
        self.enable_mouse_capture();
        self.enable_hide_cursor();
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
        self.size = Size {
            width: size.0,
            height: size.1,
        };
        self
    }

    pub fn pos(&mut self, pos: (u16, u16)) -> &mut Self {
        self.pos = Position { x: pos.0, y: pos.1 };
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

    pub fn build_render<T, W>(&self, thing: T, writer: W) -> Render<W, T>
    where
        T: Draw + Send + Sync + 'static,
        W: std::io::Write,
    {
        Render::from_builder(self, thing, writer)
    }

    pub fn build_event_loop<T>(&self, thing: T) -> EventLoop<T>
    where
        T: DrawUpdate + Send + Sync + 'static,
    {
        EventLoop::from_builder(self, thing)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
