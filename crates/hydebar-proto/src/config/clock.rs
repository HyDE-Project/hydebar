//! Configuration for the clock module.

use serde::Deserialize;

/// Clock module behaviour.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ClockModuleConfig {
    pub format:       String,
    #[serde(default)]
    pub show_weather: bool
}

impl Default for ClockModuleConfig {
    fn default() -> Self {
        Self {
            format:       "%a %d %b %R".to_string(),
            show_weather: false
        }
    }
}
