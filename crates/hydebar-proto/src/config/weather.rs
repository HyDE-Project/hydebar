//! Configuration for the weather module.

use serde::Deserialize;

/// Weather module behaviour.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WeatherModuleConfig {
    #[serde(default = "default_weather_location")]
    pub location:                String,
    pub api_key:                 Option<String>,
    #[serde(default = "default_use_celsius")]
    pub use_celsius:             bool,
    #[serde(default = "default_weather_update_interval")]
    pub update_interval_minutes: u64
}

impl Default for WeatherModuleConfig {
    fn default() -> Self {
        Self {
            location:                default_weather_location(),
            api_key:                 None,
            use_celsius:             default_use_celsius(),
            update_interval_minutes: default_weather_update_interval()
        }
    }
}

fn default_weather_location() -> String {
    String::from("London")
}

fn default_use_celsius() -> bool {
    true
}

fn default_weather_update_interval() -> u64 {
    30
}
