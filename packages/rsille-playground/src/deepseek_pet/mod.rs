//! With pet mouse interaction enabled, the session reserves SGR mouse reports for chasing
//! the pointer, petting the whale, and dragging treats. In that mode a literal escape prefix may be delayed
//! briefly while checking for a mouse report.
//! Food is inferred only from recent ASCII glyphs at known terminal positions.
#[cfg(unix)]
mod animation;
#[cfg(unix)]
mod commands;
#[cfg(unix)]
mod dialogue;
#[cfg(unix)]
mod food;
#[cfg(unix)]
mod pet;
#[cfg(unix)]
mod pursuit;
#[cfg(unix)]
mod session;

mod features;
pub use features::Feature;
use std::collections::BTreeSet;

#[derive(Clone, clap::Parser)]
pub struct Options {
    /// Program and arguments to run in the PTY instead of an interactive shell
    #[arg(last = true, value_name = "COMMAND", num_args = 1..)]
    pub command: Vec<std::ffi::OsString>,
    /// Graphics backend. Auto prefers Kitty and falls back to advertised Sixel.
    #[arg(long, default_value = "auto", value_name = "auto|kitty|sixel")]
    pub backend: rsille::render::RasterBackend,
    /// Enable only the listed behaviors (comma-separated). An empty list keeps the idle pet.
    #[arg(long, value_enum, value_delimiter = ',', num_args = 0..=1)]
    pub features: Option<Vec<Feature>>,
    /// Skip caching and restoring shell text covered by the pet
    #[arg(long)]
    pub no_restore: bool,
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
            command: Vec::new(),
            backend: rsille::render::RasterBackend::Auto,
            features: None,
            no_restore: false,
            alt_screen: false,
            commands: None,
        }
    }
}

impl Options {
    pub fn enabled_features(&self) -> BTreeSet<Feature> {
        use clap::ValueEnum;
        self.features
            .as_deref()
            .unwrap_or_else(|| Feature::value_variants())
            .iter()
            .copied()
            .collect()
    }

    #[cfg(unix)]
    pub fn run(self) -> std::io::Result<()> {
        let direct = !self.command.is_empty();
        let status = session::run(self)?;
        if direct && status != 0 {
            std::process::exit(status);
        }
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn run(self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "deepseek-pet requires a Unix terminal with PTY support",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn selects_exactly_the_requested_features() {
        use Feature::*;
        for (args, expected) in [
            (
                vec!["pet", "--features", "speech,mouse"],
                vec![Speech, Mouse],
            ),
            (
                vec!["pet", "--features", "mouse", "--features", "speech,mouse"],
                vec![Speech, Mouse],
            ),
            (vec!["pet", "--features", "speech"], vec![Speech]),
            (vec!["pet", "--features"], vec![]),
        ] {
            assert_eq!(
                Options::try_parse_from(args).unwrap().enabled_features(),
                expected.into_iter().collect()
            );
        }
        assert!(Options::try_parse_from(["pet", "--features", "drag"]).is_err());
        assert!(Options::try_parse_from(["pet", "--quite"]).is_err());
        assert_eq!(
            Options::parse_from(["pet"]).enabled_features(),
            Options::default().enabled_features()
        );
    }
}
