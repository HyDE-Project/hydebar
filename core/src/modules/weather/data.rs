//! What a weather reading is: the API answer and the value the bar shows.

use serde::Deserialize;

/// `OpenWeatherMap` API response structures
#[derive(Debug, Clone, Deserialize)]
pub struct WeatherResponse {
    /// The readings grouped under one heading.
    pub main:    MainWeather,
    /// The conditions named for the hour.
    pub weather: Vec<WeatherCondition>,
    /// What the wind is doing.
    pub wind:    Wind
}

/// The readings the service groups under one heading.
#[derive(Debug, Clone, Deserialize)]
pub struct MainWeather {
    /// Temperature, in the unit the request asked for.
    pub temp:     f64,
    /// Relative humidity, in percent.
    pub humidity: u32
}

/// One condition the service names for the hour.
#[derive(Debug, Clone, Deserialize)]
pub struct WeatherCondition {
    /// What the condition is called, in words.
    pub description: String,
    /// Key the service names its own glyph by.
    pub icon:        String
}

/// What the wind is doing.
#[derive(Debug, Clone, Deserialize)]
pub struct Wind {
    /// Wind speed, in the unit the request asked for.
    pub speed: f64
}

/// Weather data for rendering
#[derive(Debug, Clone)]
pub struct WeatherData {
    /// Temperature, written the way the bar draws it.
    pub temperature:  String,
    /// What the sky is doing, in words.
    pub description:  String,
    /// Relative humidity, written the way the bar draws it.
    pub humidity:     String,
    /// Wind speed, written the way the bar draws it.
    pub wind_speed:   String,
    /// The place the reading is for.
    pub location:     String,
    /// Whether the temperature above is in Celsius.
    pub use_celsius:  bool,
    /// When the reading arrived.
    pub last_updated: chrono::DateTime<chrono::Local>
}

/// Temperature stood in for while no reading has arrived.
///
/// The one spelling of "nothing yet": a reading that failed, one still on
/// its way and one the session has no key to ask for all wear it, and
/// [`WeatherData::has_reading`] is what tells them apart from a real sky.
const NO_READING: &str = "--";

impl WeatherData {
    /// A reading standing in for one that has not arrived.
    #[must_use]
    pub fn new(location: String, use_celsius: bool) -> Self {
        Self {
            temperature: String::from(NO_READING),
            description: String::from("Loading..."),
            humidity: String::from("--"),
            wind_speed: String::from("--"),
            location,
            use_celsius,
            last_updated: chrono::Local::now()
        }
    }

    /// Builds a reading from an API response, converting the Kelvin
    /// temperature `OpenWeatherMap` returns by default into the unit the
    /// configuration asked for.
    #[must_use]
    pub fn from_response(response: &WeatherResponse, location: String, use_celsius: bool) -> Self {
        let temp_kelvin = response.main.temp;
        let temperature = if use_celsius {
            format!("{:.0}°C", temp_kelvin - 273.15)
        } else {
            format!("{:.0}°F", (temp_kelvin - 273.15) * 9.0 / 5.0 + 32.0)
        };

        let description = response
            .weather
            .first()
            .map_or_else(|| String::from("Unknown"), |w| w.description.clone());

        Self {
            temperature,
            description,
            humidity: format!("{}%", response.main.humidity),
            wind_speed: format!("{:.1} m/s", response.wind.speed),
            location,
            use_celsius,
            last_updated: chrono::Local::now()
        }
    }

    /// Reports whether the sky was actually read.
    ///
    /// A surface with room to leave a block out — the desk above all — draws
    /// nothing rather than a placeholder nobody asked for.
    #[must_use]
    pub fn has_reading(&self) -> bool {
        self.temperature != NO_READING
    }

    /// The temperature, written the way the bar draws it.
    #[must_use]
    pub fn display_temp(&self) -> &str {
        &self.temperature
    }

    /// What the sky is doing, in words.
    #[must_use]
    pub fn display_description(&self) -> &str {
        &self.description
    }
}

/// Events emitted by the weather module
#[derive(Debug, Clone)]
pub enum WeatherEvent {
    /// A fresh reading arrived.
    Updated(WeatherData),
    /// The reading could not be taken.
    Error(String)
}

/// Message type for GUI communication
#[derive(Debug, Clone)]
pub enum Message {
    /// A fresh reading arrived.
    Update(WeatherData),
    /// The reading could not be taken.
    Error(String),
    /// Take a reading now.
    Refresh
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn freezing_point_response() -> WeatherResponse {
        WeatherResponse {
            main:    MainWeather {
                temp:     273.15,
                humidity: 50
            },
            weather: vec![WeatherCondition {
                description: String::from("clear sky"),
                icon:        String::from("01d")
            }],
            wind:    Wind {
                speed: 1.5
            }
        }
    }

    #[test]
    fn weather_data_new() {
        let data = WeatherData::new(String::from("London"), true);
        assert_eq!(data.location, "London");
        assert_eq!(data.temperature, "--");
        assert!(data.use_celsius);
    }

    #[test]
    fn weather_data_display() {
        let data = WeatherData::new(String::from("London"), true);
        assert_eq!(data.display_temp(), "--");
        assert_eq!(data.display_description(), "Loading...");
    }

    #[test]
    fn from_response_converts_the_freezing_point_to_zero_celsius() {
        let data =
            WeatherData::from_response(&freezing_point_response(), String::from("London"), true);
        assert_eq!(data.temperature, "0°C");
        assert_eq!(data.humidity, "50%");
        assert_eq!(data.wind_speed, "1.5 m/s");
    }

    #[test]
    fn from_response_converts_the_freezing_point_to_thirty_two_fahrenheit() {
        let data =
            WeatherData::from_response(&freezing_point_response(), String::from("London"), false);
        assert_eq!(data.temperature, "32°F");
        assert!(!data.use_celsius);
    }
}
