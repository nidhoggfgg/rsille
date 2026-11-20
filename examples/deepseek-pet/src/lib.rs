//! With pet mouse interaction enabled, the session reserves SGR mouse reports for chasing
//! the pointer and petting the whale. In that mode a literal escape prefix may be delayed
//! briefly and pasted reports look like mouse input.
//! Food is inferred only from recent ASCII glyphs at known terminal positions.
#[cfg(unix)]
mod boundary;
#[cfg(unix)]
mod commands;
#[cfg(unix)]
mod dialogue;
#[cfg(unix)]
mod food;
#[cfg(unix)]
mod mouse;
#[cfg(unix)]
mod pet;
#[cfg(unix)]
mod session;

pub fn run() -> std::io::Result<()> {
    Options::default().run()
}

pub fn run_with_mouse_capture(capture_mouse: bool) -> std::io::Result<()> {
    Options {
        capture_mouse,
        ..Options::default()
    }
    .run()
}

#[derive(Clone, clap::Parser)]
pub struct Options {
    /// Disable pet mouse capture
    #[arg(long = "quite", action = clap::ArgAction::SetFalse)]
    pub capture_mouse: bool,
    /// Run the pet on the alternate screen
    #[arg(long)]
    pub alt_screen: bool,
    /// Load custom command replies from a YAML file
    #[arg(long, value_name = "FILE")]
    pub commands: Option<std::path::PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            capture_mouse: true,
            alt_screen: false,
            commands: None,
        }
    }
}

impl Options {
    #[cfg(unix)]
    pub fn run(self) -> std::io::Result<()> {
        session::run(self)
    }

    #[cfg(not(unix))]
    pub fn run(self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "deepseek-pet requires a Unix terminal with PTY support",
        ))
    }
}
