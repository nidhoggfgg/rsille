//! With pet mouse interaction enabled, the session reserves SGR mouse reports for chasing
//! the pointer and petting the whale. In that mode a literal escape prefix may be delayed
//! briefly and pasted reports look like mouse input.
//! Food is inferred only from recent ASCII glyphs at known terminal positions.
#[cfg(unix)]
mod boundary;
#[cfg(unix)]
mod food;
#[cfg(unix)]
mod mouse;
#[cfg(unix)]
mod pet;
#[cfg(unix)]
mod session;

pub fn run() -> std::io::Result<()> {
    run_with_mouse_capture(false)
}

#[cfg(unix)]
pub fn run_with_mouse_capture(capture_mouse: bool) -> std::io::Result<()> {
    session::run(crate::pet::Pet::default(), capture_mouse)
}

#[cfg(not(unix))]
pub fn run_with_mouse_capture(_capture_mouse: bool) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "deepseek-pet requires a Unix terminal with PTY support",
    ))
}
