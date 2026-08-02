//! What a weather reading is: the API answer and the value the bar shows.

use serde::Deserialize;

/// `OpenWeatherMap` API response structures
#[derive(Debug, Clone, Deserialize)]
pub struct WeatherResponse {
    pub main:    MainWeather,
    pub weather: Vec<WeatherCondition>,
    pub wind:    Wind
}

#[derive(Debug, Clone, Deserialize)]
pub struct MainWeather {
    pub temp:     f64,
    pub humidity: u32
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeatherCondition {
    pub description: String,
    pub icon:        String
}

#[derive(Debug, Clone, Deserialize)]
pub struct Wind {
    pub speed: f64
}

/// Weather data for rendering
#[derive(Debug, Clone)]
pub struct WeatherData {
    pub temperature:  String,
    pub description:  String,
    pub humidity:     String,
    pub wind_speed:   String,
    pub location:     String,
    pub use_celsius:  bool,
    pub last_updated: chrono::DateTime<chrono::Local>
}

impl WeatherData {
    #[must_use]
    pub fn new(location: String, use_celsius: bool) -> Self {
        Self {
            temperature: String::from("--"),
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

    #[must_use]
    pub fn display_temp(&self) -> &str {
        &self.temperature
    }

    #[must_use]
    pub fn display_description(&self) -> &str {
        &self.description
    }
}

/// Events emitted by the weather module
#[derive(Debug, Clone)]
pub enum WeatherEvent {
    Updated(WeatherData),
    Error(String)
}

/// Message type for GUI communication
#[derive(Debug, Clone)]
pub enum Message {
    Update(WeatherData),
    Error(String),
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
