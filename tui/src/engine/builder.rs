use futures::io;
use term::crossterm::event::{KeyCode, KeyEvent};

use super::TuiEngine;

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
        }
    }

    // (col, row) aka (x, y)
    pub fn set_size(&mut self, (col, row): (u16, u16)) -> &mut Self {
        self.size = Size::Fixed(col, row);
        self
    }

    pub fn full_screen(&mut self) -> &mut Self {
        self.size = Size::Auto;
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

    pub fn set_max_event_per_frame(&mut self, max_event_per_frame: usize) -> &mut Self {
        self.max_event_per_frame = max_event_per_frame;
        self
    }

    pub fn set_frame_limit(&mut self, frame_limit: u16) -> &mut Self {
        self.frame_limit = Some(frame_limit);
        self
    }

    pub fn build(&self) -> io::Result<TuiEngine> {
        TuiEngine::from_builder(self)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub(super) enum Size {
    Fixed(u16, u16),
    FullScreen,

    // unimplemented, bc panel have fixed size
    Auto,
}

// unimplemented
// this is useful, when want to use TuiEngine with other things
// like put a clock on the right top corner in shell
#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub(super) enum Position {
    LeftTop,
    RightTop,
    LeftBottom,
    RightBottom,
    Center,
}
