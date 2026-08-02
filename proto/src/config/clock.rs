//! Configuration for the clock module.

use serde::Deserialize;

/// Clock module behaviour.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ClockModuleConfig {
    /// Primary `chrono` format string.
    pub format:       String,
    /// Alternative format strings a left click cycles through.
    ///
    /// Leaving the list empty keeps the module on [`Self::format`] forever, so
    /// a configuration written before alternatives existed behaves as before.
    #[serde(default, alias = "format-alt")]
    pub format_alt:   Vec<String>,
    #[serde(default)]
    pub show_weather: bool
}

impl ClockModuleConfig {
    /// Reports whether a left click has another format to switch to.
    #[must_use]
    pub const fn has_alternatives(&self) -> bool {
        !self.format_alt.is_empty()
    }

    /// Every format the module can render, primary format first.
    pub fn formats(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.format.as_str()).chain(self.format_alt.iter().map(String::as_str))
    }
}

impl Default for ClockModuleConfig {
    fn default() -> Self {
        Self {
            format:       "%a %d %b %R".to_string(),
            format_alt:   Vec::new(),
            show_weather: false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_config_without_alternatives_offers_only_its_format() {
        let config = ClockModuleConfig::default();

        assert!(!config.has_alternatives());
        assert_eq!(config.formats().collect::<Vec<_>>(), vec!["%a %d %b %R"]);
    }

    #[test]
    fn alternatives_follow_the_primary_format() {
        let config: ClockModuleConfig = toml::from_str(
            r#"
            format = "%I:%M %p"
            format-alt = ["%R %d.%m.%y"]
            "#
        )
        .expect("clock config");

        assert!(config.has_alternatives());
        assert_eq!(
            config.formats().collect::<Vec<_>>(),
            vec!["%I:%M %p", "%R %d.%m.%y"]
        );
    }

    #[test]
    fn the_snake_case_key_is_accepted_as_well() {
        let config: ClockModuleConfig = toml::from_str(
            r#"
            format = "%R"
            format_alt = ["%T"]
            "#
        )
        .expect("clock config");

        assert_eq!(config.format_alt, vec!["%T".to_string()]);
    }
}
