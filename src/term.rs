//! Some useful function to print styles in terminal
//！
//! ## NOTE:
//!
//! Many functions put some escape sequences to the stdout,
//! so use them only when you know what you are doing

use crossterm::{cursor, execute, terminal};

/// Get the *(width, height)* of terminal
///
/// If can't get the size, return *(80, 24)*
pub fn get_terminal_size() -> (u16, u16) {
    terminal::size().unwrap_or((80, 24))
}

/// Check the terminal is in raw mode or not
///
/// If can't get the mode, return false
pub fn is_raw_mode() -> bool {
    terminal::is_raw_mode_enabled().unwrap_or(false)
}

/// Clear the screen
pub fn clear() {
    execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();
}

/// Hide the cursor
pub fn hide_cursor() {
    execute!(std::io::stdout(), cursor::Hide).unwrap();
}

/// Show the cursor
pub fn show_cursor() {
    execute!(std::io::stdout(), cursor::Show).unwrap();
}

/// Move cursor to *(x, y)*
pub fn move_to(x: u32, y: u32) {
    execute!(std::io::stdout(), cursor::MoveTo(x as u16, y as u16)).unwrap();
}

/// When output is longer than the terminal width, put the rest output in next line (default)
pub fn enable_wrap() {
    execute!(std::io::stdout(), terminal::EnableLineWrap).unwrap();
}

/// When output is longer than the terminal width, don't put the rest output in next line
pub fn disable_wrap() {
    execute!(std::io::stdout(), terminal::DisableLineWrap).unwrap();
}
