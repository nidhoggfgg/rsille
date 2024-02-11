//! some useful function to print styles in terminal

#[cfg(feature = "termsize")]
use crate::utils::RsilleErr;

/// get the (width, height) of terminal
#[cfg(feature = "termsize")]
pub fn get_terminal_size() -> Result<(usize, usize), RsilleErr> {
    use std::mem;
    unsafe {
        let mut winsize: libc::winsize = mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize) == 0 {
            Ok((winsize.ws_col as usize, winsize.ws_row as usize))
        } else {
            Err(RsilleErr::new("can't get terminal size".to_string()))
        }
    }
}

/// clear the screen
pub fn clear() {
    print!("\x1B[2J\x1B[H");
}

/// hide the cursor
///
/// not defined by the specification, may not work in multiplexers like tmux
pub fn hide_cursor() {
    print!("\x1B[?25l");
}

/// show the cursor
///
/// not defined by the specification, may not work in multiplexers like tmux
pub fn show_cursor() {
    print!("\x1B[?25h");
}

/// move cursor to (x, y)
pub fn move_to(x: u32, y: u32) {
    if x == 0 && y == 0 {
        print!("\x1B[H");
    } else {
        print!("\x1B[{y};{x}H");
    }
}

/// when output is longer than the terminal width, put the rest output in next line (default)
///
/// not defined by the specification, may not work in multiplexers like tmux
pub fn enable_wrap() {
    print!("\x1B[?7h");
}

/// when output is longer than the terminal width, don't put the rest output in next line
///
/// not defined by the specification, may not work in multiplexers like tmux
pub fn disable_wrap() {
    print!("\x1B[?7l");
}
