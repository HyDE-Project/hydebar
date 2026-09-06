//! Preset themes the configuration can name instead of spelling a whole
//! appearance out by hand.
//!
//! Each preset is a TOML file of its own stating a whole appearance table;
//! this module keeps the roster and the serde bridge that accepts either a
//! preset name or a full appearance table. Adding a theme means adding a
//! file and one row to [`THEME_SOURCES`]: no Rust logic changes.

use log::warn;
use serde::{Deserialize, Deserializer, de::Error as _};

use super::appearance::Appearance;

/// The preset theme files shipped with the bar, keyed by kebab-case name.
const THEME_SOURCES: &[(&str, &str)] = &[
    ("catppuccin-frappe", include_str!("catppuccin-frappe.toml")),
    ("catppuccin-latte", include_str!("catppuccin-latte.toml")),
    (
        "catppuccin-macchiato",
        include_str!("catppuccin-macchiato.toml")
    ),
    ("catppuccin-mocha", include_str!("catppuccin-mocha.toml")),
    ("dracula", include_str!("dracula.toml")),
    ("gruvbox-dark", include_str!("gruvbox-dark.toml")),
    ("gruvbox-light", include_str!("gruvbox-light.toml")),
    ("nord", include_str!("nord.toml")),
    ("tokyo-night", include_str!("tokyo-night.toml")),
    ("tokyo-night-light", include_str!("tokyo-night-light.toml")),
    ("tokyo-night-storm", include_str!("tokyo-night-storm.toml"))
];

/// Names every preset theme the bar ships with, in kebab-case.
///
/// # Examples
///
/// ```
/// use hydebar_proto::config::{preset_appearance, preset_names};
///
/// assert!(preset_names().contains(&"dracula"));
/// assert!(preset_appearance("no-such-theme").is_none());
/// ```
#[must_use]
pub fn preset_names() -> Vec<&'static str> {
    THEME_SOURCES.iter().map(|(name, _)| *name).collect()
}

/// Reads the named preset into the appearance it stands for.
///
/// Returns [`None`] when no shipped theme answers to `name`; the caller —
/// usually the configuration loader, which also consults the user's own
/// theme files — turns that into an error naming what is available.
///
/// A preset file failing to parse means a corrupted build: the files ship
/// inside the binary and the configuration tests pin every value they hold,
/// so the bar answers with its default appearance instead of refusing to
/// start.
///
/// # Examples
///
/// ```
/// use hydebar_proto::config::preset_appearance;
///
/// let mocha = preset_appearance("catppuccin-mocha").expect("shipped preset");
/// assert!(mocha.follow_hyde);
/// ```
#[must_use]
pub fn preset_appearance(name: &str) -> Option<Appearance> {
    let (_, source) = THEME_SOURCES.iter().find(|(known, _)| *known == name)?;

    match toml::from_str(source) {
        Ok(appearance) => Some(appearance),
        Err(error) => {
            warn!("embedded theme '{name}' failed to parse ({error}); using defaults");
            Some(Appearance::default())
        }
    }
}

/// Accepts either a preset name or a whole appearance table.
///
/// # Errors
///
/// Returns the deserializer's own error when the value is neither a preset
/// name the bar knows nor a readable appearance table. An unknown name
/// answers with the shipped names, so a misspelling says what was meant.
pub fn deserialize_theme_or_appearance<'de, D>(deserializer: D) -> Result<Appearance, D::Error>
where
    D: Deserializer<'de>
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ThemeOrAppearance {
        Theme(String),
        Appearance(Box<Appearance>)
    }

    match ThemeOrAppearance::deserialize(deserializer)? {
        ThemeOrAppearance::Theme(name) => preset_appearance(&name).ok_or_else(|| {
            D::Error::custom(format!(
                "unknown theme '{name}'; shipped themes are: {}",
                preset_names().join(", ")
            ))
        }),
        ThemeOrAppearance::Appearance(appearance) => Ok(*appearance)
    }
}
