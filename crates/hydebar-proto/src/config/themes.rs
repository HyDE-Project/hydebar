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
    CatppuccinMocha,
    CatppuccinMacchiato,
    CatppuccinFrappe,
    CatppuccinLatte,
    Dracula,
    Nord,
    GruvboxDark,
    GruvboxLight,
    TokyoNight,
    TokyoNightStorm,
    TokyoNightLight
}

impl PresetTheme {
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
