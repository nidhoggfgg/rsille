use term::crossterm::event::{KeyCode, KeyEvent};

use crate::{DrawUpdate, Render, event_loop::EventLoop, render::Size};

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
    pub(super) home: (u16, u16),
}

impl Builder {
    pub fn new() -> Self {
        Self {
            enable_raw_mode: false,
            enable_alt_screen: false,
            enable_mouse_capture: false,
            enable_hide_cursor: false,
            exit_code: KeyCode::Esc.into(),
            frame_limit: None,
            max_event_per_frame: 10,
            size: Size::FullScreen,
            home: (0, 0),
        }
    }

    // (col, row) aka (x, y)
    pub fn size(&mut self, (col, row): (u16, u16)) -> &mut Self {
        self.size = Size::Fixed(col, row);
        self
    }

    pub fn full_screen(&mut self) -> &mut Self {
        self.size = Size::FullScreen;
        self
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

    pub fn home(&mut self, home: (u16, u16)) -> &mut Self {
        self.home = home;
        self
    }

    pub fn build_eventloop<T>(&self, thing: T) -> EventLoop
    where
        T: DrawUpdate + Send + Sync + 'static,
    {
        EventLoop::from_builder(self, thing)
    }

    pub fn build_render<T, W>(&self, thing: T, writer: W) -> Render<W>
    where
        T: DrawUpdate + Send + Sync + 'static,
        W: std::io::Write,
    {
        Render::from_builder(self, thing, writer)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
