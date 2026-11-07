//! Terminal input configuration and the capabilities actually enabled at startup.

/// Keyboard protocol policy. Existing applications keep legacy input by default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum KeyboardMode {
    /// Uses the platform's default key reporting without protocol negotiation.
    #[default]
    Legacy,
    /// Negotiates full key reporting in raw mode when not provided natively.
    ///
    /// An unsupported terminal or failed capability query falls back to legacy
    /// input. Failure to enable a supported protocol is a terminal setup error.
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
