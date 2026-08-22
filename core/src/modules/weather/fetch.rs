//! The round-trip to the `OpenWeatherMap` API, errors spelled for the bar.

use masterror::{AppError, AppResult};

use super::data::WeatherResponse;

/// Spells `value` safely inside one URL query component.
///
/// The location comes from the configuration file; an ampersand or a hash
/// in it must read as part of the place name, never as query syntax.
fn percent_encoded(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            other => {
                use std::fmt::Write;

                let _ = write!(encoded, "%{other:02X}");
            }
        }
    }

    encoded
}

/// Fetch weather data from `OpenWeatherMap` API
pub(super) async fn fetch_weather(
    location: &str,
    api_key: Option<&String>
) -> AppResult<WeatherResponse> {
    let api_key = api_key
        .ok_or_else(|| AppError::config("Weather API key not configured in config.toml"))?;

    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}",
        percent_encoded(location),
        percent_encoded(api_key)
    );

    let response = crate::utils::http_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                AppError::timeout(format!("Weather API timeout for location '{location}'"))
            } else if e.is_connect() {
                AppError::service_unavailable("No internet connection - cannot fetch weather")
            } else {
                AppError::external_api(format!("Network error fetching weather: {e}"))
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 => AppError::unauthorized(format!("Invalid weather API key ({status})")),
            404 => AppError::not_found(format!(
                "Location '{location}' not found in weather database"
            )),
            429 => AppError::rate_limited("Weather API rate limit exceeded - try again later"),
            500..=599 => {
                AppError::service_unavailable(format!("Weather API server error ({status})"))
            }
            _ => AppError::external_api(format!(
                "Weather API returned error {status} for location '{location}'"
            ))
        });
    }

    let weather = response.json::<WeatherResponse>().await.map_err(|e| {
        AppError::deserialization(format!("Invalid weather data format from API: {e}"))
    })?;

    Ok(weather)
}
