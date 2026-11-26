use std::ffi::{OsStr, OsString};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(super) use unix::*;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(super) use windows::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
    #[cfg(unix)]
    pub xpixel: u16,
    #[cfg(unix)]
    pub ypixel: u16,
}

pub struct Spawn<'a> {
    pub program: &'a OsStr,
    pub args: &'a [OsString],
    pub size: Size,
    #[cfg(unix)]
    pub direct: bool,
    #[cfg(unix)]
    pub name: &'a str,
}

#[derive(Default)]
pub struct Ready {
    pub input: bool,
    pub output: bool,
    pub writable: bool,
}
