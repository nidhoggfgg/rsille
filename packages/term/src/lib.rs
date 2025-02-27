pub mod event;
pub mod style;

pub extern crate crossterm;
pub use fns::*;

mod fns {
    use crossterm::{
        cursor::{Hide, Show},
        event::{DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    };
    use std::io;
    pub fn enable_raw_mode() -> io::Result<()> {
        crossterm::terminal::enable_raw_mode()
    }

    pub fn disable_raw_mode() -> io::Result<()> {
        crossterm::terminal::disable_raw_mode()
    }

    pub fn enable_mouse_capture() -> io::Result<()> {
        execute!(io::stdout(), EnableMouseCapture)
    }

    pub fn disable_mouse_capture() -> io::Result<()> {
        execute!(io::stdout(), DisableMouseCapture)
    }

    pub fn enable_focus_change() -> io::Result<()> {
        execute!(io::stdout(), EnableFocusChange)
    }

    pub fn hide_cursor() -> io::Result<()> {
        execute!(io::stdout(), Hide)
    }

    pub fn show_cursor() -> io::Result<()> {
        execute!(io::stdout(), Show)
    }

    pub fn enter_alt_screen() -> io::Result<()> {
        execute!(io::stdout(), EnterAlternateScreen)
    }

    pub fn leave_alt_screen() -> io::Result<()> {
        execute!(io::stdout(), LeaveAlternateScreen)
    }

    pub fn terminal_size() -> io::Result<(u16, u16)> {
        let (w, h) = crossterm::terminal::size()?;
        Ok((w, h))
    }
}
