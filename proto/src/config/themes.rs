//! Preset themes the configuration can name instead of spelling a whole
//! appearance out by hand.
//!
//! Each palette lives in a module of its own, named after the theme it
//! dresses the bar in; this file keeps the roster and the serde bridge that
//! accepts either a preset name or a full appearance table.

mod catppuccin_frappe;
mod catppuccin_latte;
mod catppuccin_macchiato;
mod catppuccin_mocha;
mod dracula;
mod gruvbox_dark;
mod gruvbox_light;
mod nord;
mod tokyo_night;
mod tokyo_night_light;
mod tokyo_night_storm;

use serde::{Deserialize, Deserializer};

use super::appearance::Appearance;

/// The preset themes shipped with the bar.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PresetTheme {
    /// The darkest of the Catppuccin palettes.
    CatppuccinMocha,
    /// A dark Catppuccin palette, warmer than mocha.
    CatppuccinMacchiato,
    /// The lightest of the dark Catppuccin palettes.
    CatppuccinFrappe,
    /// The light Catppuccin palette.
    CatppuccinLatte,
    /// The Dracula palette.
    Dracula,
    /// The Nord palette.
    Nord,
    /// The dark Gruvbox palette.
    GruvboxDark,
    /// The light Gruvbox palette.
    GruvboxLight,
    /// The Tokyo Night palette.
    TokyoNight,
    /// The Tokyo Night palette in its storm shade.
    TokyoNightStorm,
    /// The light Tokyo Night palette.
    TokyoNightLight
}

impl PresetTheme {
    /// The full appearance the named preset stands for.
    #[must_use]
    pub fn to_appearance(self) -> Appearance {
        match self {
            Self::CatppuccinMocha => catppuccin_mocha::catppuccin_mocha(),
            Self::CatppuccinMacchiato => catppuccin_macchiato::catppuccin_macchiato(),
            Self::CatppuccinFrappe => catppuccin_frappe::catppuccin_frappe(),
            Self::CatppuccinLatte => catppuccin_latte::catppuccin_latte(),
            Self::Dracula => dracula::dracula(),
            Self::Nord => nord::nord(),
            Self::GruvboxDark => gruvbox_dark::gruvbox_dark(),
            Self::GruvboxLight => gruvbox_light::gruvbox_light(),
            Self::TokyoNight => tokyo_night::tokyo_night(),
            Self::TokyoNightStorm => tokyo_night_storm::tokyo_night_storm(),
            Self::TokyoNightLight => tokyo_night_light::tokyo_night_light()
        }
    }
}

/// Accepts either a preset name or a whole appearance table.
///
/// # Errors
///
/// Returns the deserializer's own error when the value is neither a name the
/// bar knows nor a readable appearance table.
pub fn deserialize_theme_or_appearance<'de, D>(deserializer: D) -> Result<Appearance, D::Error>
where
    D: Deserializer<'de>
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ThemeOrAppearance {
        Theme(PresetTheme),
        Appearance(Box<Appearance>)
    }

    match ThemeOrAppearance::deserialize(deserializer)? {
        ThemeOrAppearance::Theme(theme) => Ok(theme.to_appearance()),
        ThemeOrAppearance::Appearance(appearance) => Ok(*appearance)
    }
}
