//! Inline terminal artwork from the ai-babies example.
mod static_image;

use std::{error::Error, io::IsTerminal};

#[doc(hidden)]
pub use static_image::command as static_command;

/// Draw the embedded static trio once. Redirected stdout is left untouched.
pub fn run() -> Result<(), Box<dyn Error>> {
    if !std::io::stdout().is_terminal() {
        return Ok(());
    }
    let (width, height) = crossterm::terminal::size()?;
    let rows = (width / 6).min(height).min(24);
    if rows > 0 {
        static_image::render(rows)?;
    }
    Ok(())
}
