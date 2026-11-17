//! CLI preset names shared by command-line arguments and YAML configuration.
use clap::ValueEnum;
use rsille::tui::prelude::Theme;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreset {
    /// Neutral dark background with blue accents (default)
    #[default]
    Dark,
    /// Light background with indigo accents
    Light,
    /// Muted charcoal with blue and cyan accents
    OneDark,
    /// Dark purple with vivid violet and cyan accents
    Dracula,
    /// Deep navy with cool blue accents
    TokyoNight,
}

impl ThemePreset {
    pub fn build(self) -> Theme {
        let mut theme = match self {
            Self::Dark => Theme::dark(),
            Self::Light => Theme::light(),
            Self::OneDark => Theme::one_dark(),
            Self::Dracula => Theme::dracula(),
            Self::TokyoNight => Theme::tokyo_night(),
        };
        // CLI prompts share one background across labels, layouts and controls.
        theme.styles.surface.bg_color = theme.styles.surface_elevated.bg_color;
        theme
    }
}
