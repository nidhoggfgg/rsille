//! Structural appearance, independent of the color theme.
use clap::ValueEnum;
use rsille::tui::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum StylePreset {
    /// Original compact controls with single-line borders
    #[default]
    Classic,
    /// Borderless lists, underlined inputs and text buttons
    Minimal,
    /// Rounded borders, generous spacing and filled buttons
    Rounded,
    /// Double-line borders and outlined buttons
    Retro,
    /// Heavy borders and prominent filled buttons
    Bold,
}

impl StylePreset {
    pub fn border(self) -> BorderStyle {
        match self {
            Self::Classic | Self::Minimal => BorderStyle::Single,
            Self::Rounded => BorderStyle::Rounded,
            Self::Retro => BorderStyle::Double,
            Self::Bold => BorderStyle::Thick,
        }
    }

    pub fn gap(self) -> u16 {
        match self {
            Self::Classic | Self::Minimal => 0,
            _ => 1,
        }
    }

    pub fn padding(self) -> Padding {
        match self {
            Self::Rounded => Padding::new(1, 2, 1, 2),
            Self::Retro | Self::Bold => Padding::horizontal(1),
            _ => Padding::ZERO,
        }
    }

    pub fn button(self) -> ButtonVariant {
        match self {
            Self::Minimal => ButtonVariant::Link,
            Self::Retro => ButtonVariant::Ghost,
            _ => ButtonVariant::Primary,
        }
    }
}
