#[cfg(unix)]
mod boundary;
#[cfg(unix)]
mod pet;
#[cfg(unix)]
mod session;

#[cfg(unix)]
fn main() -> std::io::Result<()> {
    session::run(crate::pet::Pet::default())
}

#[cfg(not(unix))]
fn main() {
    eprintln!("deepseek-pet requires a Unix terminal with PTY support");
    std::process::exit(1);
}
