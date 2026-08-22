//! Configuration for the weather module.

use serde::Deserialize;

/// Weather module behaviour.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WeatherModuleConfig {
    /// The place the weather is read for.
    #[serde(default = "default_weather_location")]
    pub location:                String,
    /// The key the weather service is asked with, where one is needed.
    pub api_key:                 Option<String>,
    /// Whether the temperature is written in Celsius rather than Fahrenheit.
    #[serde(default = "default_use_celsius")]
    pub use_celsius:             bool,
    /// Minutes between two readings.
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

const fn default_use_celsius() -> bool {
    true
}

const fn default_weather_update_interval() -> u64 {
    30
}
