use std::io;

use crossterm::{
    event::{EnableFocusChange, EnableMouseCapture},
    execute,
};

pub extern crate crossterm;

pub fn enable_raw_mode() -> io::Result<()> {
    crossterm::terminal::enable_raw_mode()
}

pub fn disable_raw_mode() -> io::Result<()> {
    crossterm::terminal::disable_raw_mode()
}

pub fn enable_mouse_capture<T: io::Write>(buffer: &mut T) -> io::Result<()> {
    execute!(buffer, EnableMouseCapture)
}

pub fn enable_focus_change<T: io::Write>(buffer: &mut T) -> io::Result<()> {
    execute!(buffer, EnableFocusChange)
}
