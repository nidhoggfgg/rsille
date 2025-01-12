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

pub fn enable_mouse_capture() -> io::Result<()> {
    execute!(io::stdout(), EnableMouseCapture)
}

pub fn enable_focus_change() -> io::Result<()> {
    execute!(io::stdout(), EnableFocusChange)
}
