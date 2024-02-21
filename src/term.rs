//! Some useful function to print styles in terminal
//！
//! ## NOTE:
//!
//! Many functions put some escape sequences to the stdout,
//! so use them only when you know what you are doing

use crossterm::{cursor, execute, terminal};

use crate::utils::{to_rsille_err, RsilleErr};

/// get the (width, height) of terminal
pub fn get_terminal_size() -> Result<(u16, u16), RsilleErr> {
    terminal::size().map_err(|e| RsilleErr::new(format!("can't get terminal size: {}", e)))
}

/// check the terminal is in raw mode or not
pub fn is_raw_mode() -> Result<bool, RsilleErr> {
    terminal::is_raw_mode_enabled().map_err(to_rsille_err)
}

/// clear the screen
pub fn clear() {
    execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();
}

/// hide the cursor
pub fn hide_cursor() {
    execute!(std::io::stdout(), cursor::Hide).unwrap();
}

/// show the cursor
pub fn show_cursor() {
    execute!(std::io::stdout(), cursor::Show).unwrap();
}

/// move cursor to (x, y)
pub fn move_to(x: u32, y: u32) {
    execute!(std::io::stdout(), cursor::MoveTo(x as u16, y as u16)).unwrap();
}

/// when output is longer than the terminal width, put the rest output in next line (default)
pub fn enable_wrap() {
    execute!(std::io::stdout(), terminal::EnableLineWrap).unwrap();
}

/// when output is longer than the terminal width, don't put the rest output in next line
pub fn disable_wrap() {
    execute!(std::io::stdout(), terminal::DisableLineWrap).unwrap();
}
