//! Terminal input configuration and the capabilities actually enabled at startup.

/// Keyboard protocol policy. Existing applications keep legacy input by default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum KeyboardMode {
    #[default]
    Legacy,
    /// Negotiate full key reporting, falling back to legacy input on failure.
    Auto,
}

/// Input features enabled for this run, rather than inferred from `$TERM`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputCapabilities {
    /// The input backend reports releases, including ordinary character keys.
    pub key_releases: bool,
    /// Focus reporting was enabled. Actual delivery also depends on the terminal.
    pub focus_events: bool,
}
