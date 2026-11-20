//! With pet mouse interaction enabled, the session reserves SGR mouse reports for chasing
//! the pointer and petting the whale. In that mode a literal escape prefix may be delayed
//! briefly and pasted reports look like mouse input.
//! Food is inferred only from recent ASCII glyphs at known terminal positions.
#[cfg(unix)]
mod animation;
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

mod features;
pub use features::Feature;
use std::collections::BTreeSet;

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
    /// Enable only the listed behaviors (comma-separated). An empty list keeps the idle pet.
    #[arg(long, value_enum, value_delimiter = ',', num_args = 0..=1)]
    pub features: Option<Vec<Feature>>,
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
            features: None,
            capture_mouse: true,
            alt_screen: false,
            commands: None,
        }
    }
}

impl Options {
    pub fn enabled_features(&self) -> BTreeSet<Feature> {
        use Feature::*;
        let mut enabled = self
            .features
            .as_deref()
            .unwrap_or(&[Roam, Mouse, Speech, CommandReactions, Typing, Bell, Forage])
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if !self.capture_mouse {
            enabled.remove(&Mouse);
        }
        enabled
    }

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
            (
                vec!["pet", "--features", "speech,mouse", "--quite"],
                vec![Speech],
            ),
            (vec!["pet", "--features"], vec![]),
        ] {
            assert_eq!(
                Options::try_parse_from(args).unwrap().enabled_features(),
                expected.into_iter().collect()
            );
        }
        assert!(Options::try_parse_from(["pet", "--features", "unknown"]).is_err());
        assert_eq!(
            Options::parse_from(["pet"]).enabled_features(),
            Options::default().enabled_features()
        );
    }
}
