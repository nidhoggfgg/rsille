//! The session reserves SGR mouse reports for chasing the pointer and petting the whale. A literal
//! escape prefix may be delayed briefly; pasted reports look like mouse input.
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

#[cfg(unix)]
pub fn run() -> std::io::Result<()> {
    session::run(crate::pet::Pet::default())
}

#[cfg(not(unix))]
pub fn run() -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "deepseek-pet requires a Unix terminal with PTY support",
    ))
}
