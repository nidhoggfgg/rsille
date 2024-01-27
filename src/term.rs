use crate::utils::RsilleErr;
use libc;
use std::mem;

pub fn get_terminal_size() -> Result<(usize, usize), RsilleErr> {
    unsafe {
        let mut winsize: libc::winsize = mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize) == 0 {
            Ok((winsize.ws_col as usize, winsize.ws_row as usize))
        } else {
            Err(RsilleErr::new("can't get terminal size".to_string()))
        }
    }
}
