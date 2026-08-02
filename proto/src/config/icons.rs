//! User supplied replacements for the built in glyphs.
//!
//! The `[icons]` table maps a built in icon name to the glyph that should be
//! rendered in its place:
//!
//! ```toml
//! [icons]
//! cpu = "\U000F035B"
//! mem = "\U000F0F86"
//! ```
//!
//! Names that do not match a built in icon are ignored, and icons without an
//! entry keep the glyph compiled into the binary.

use std::collections::HashMap;

use serde::Deserialize;

/// Glyph overrides keyed by the built in icon name.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(transparent)]
pub struct IconsConfig {
    overrides: HashMap<String, String>
}

impl IconsConfig {
    /// Returns the glyph configured for `name`, when the user declared one.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.overrides.get(name).map(String::as_str)
    }

    /// Iterates over every configured `(name, glyph)` pair.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.overrides
            .iter()
            .map(|(name, glyph)| (name.as_str(), glyph.as_str()))
    }

    /// Returns `true` when no override has been declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.overrides.is_empty()
    }

    /// Declares an override for `name`, replacing any previous value.
    pub fn insert(&mut self, name: impl Into<String>, glyph: impl Into<String>) {
        self.overrides.insert(name.into(), glyph.into());
    }
}

impl<K, V> FromIterator<(K, V)> for IconsConfig
where
    K: Into<String>,
    V: Into<String>
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self {
            overrides: iter
                .into_iter()
                .map(|(name, glyph)| (name.into(), glyph.into()))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(default)]
        icons: IconsConfig
    }

    #[test]
    fn missing_table_yields_no_overrides() {
        let wrapper: Wrapper = toml::from_str("").expect("empty config should parse");

        assert!(wrapper.icons.is_empty());
        assert_eq!(wrapper.icons.get("cpu"), None);
    }

    #[test]
    fn table_entries_become_overrides() {
        let wrapper: Wrapper = toml::from_str(
            r#"
            [icons]
            cpu = "X"
            mem = "Y"
            "#
        )
        .expect("icons table should parse");

        assert_eq!(wrapper.icons.get("cpu"), Some("X"));
        assert_eq!(wrapper.icons.get("mem"), Some("Y"));
        assert_eq!(wrapper.icons.get("temp"), None);
    }

    #[test]
    fn root_config_exposes_the_icons_table() {
        let config: crate::config::Config = toml::from_str(
            r#"
            [icons]
            cpu = "\U000F035B"
            mem = "\U000F0F86"
            "#
        )
        .expect("root config with icons table should parse");

        assert_eq!(config.icons.get("cpu"), Some("\u{f035b}"));
        assert_eq!(config.icons.get("mem"), Some("\u{f0f86}"));
    }

    #[test]
    fn root_config_without_icons_table_has_no_overrides() {
        let config: crate::config::Config =
            toml::from_str("").expect("empty root config should parse");

        assert!(config.icons.is_empty());
    }

    #[test]
    fn from_iter_builds_overrides() {
        let config = IconsConfig::from_iter([("cpu", "X")]);

        assert_eq!(config.get("cpu"), Some("X"));
        assert_eq!(config.iter().count(), 1);
    }
}
