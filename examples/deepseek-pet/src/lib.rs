#[cfg(unix)]
mod boundary;
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
