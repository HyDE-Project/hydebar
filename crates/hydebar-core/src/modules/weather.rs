use std::time::Duration;

use hydebar_proto::config::WeatherModuleConfig;
use log::{error, warn};
use masterror::{AppError, AppResult};
use serde::Deserialize;
use tokio::{runtime::Handle, task::JoinHandle, time::interval};

use crate::{
    ModuleContext, ModuleEventSender,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

/// OpenWeatherMap API response structures
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
    /// temperature OpenWeatherMap returns by default into the unit the
    /// configuration asked for.
    pub fn from_response(response: WeatherResponse, location: String, use_celsius: bool) -> Self {
        let temp_kelvin = response.main.temp;
        let temperature = if use_celsius {
            format!("{:.0}°C", temp_kelvin - 273.15)
        } else {
            format!("{:.0}°F", (temp_kelvin - 273.15) * 9.0 / 5.0 + 32.0)
        };

        let description = response
            .weather
            .first()
            .map(|w| w.description.clone())
            .unwrap_or_else(|| String::from("Unknown"));

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

    pub fn display_temp(&self) -> &str {
        &self.temperature
    }

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

/// Weather module - business logic only, no GUI!
#[derive(Debug)]
pub struct Weather {
    data:            WeatherData,
    api_key:         Option<String>,
    update_interval: Duration,
    sender:          Option<ModuleEventSender<WeatherEvent>>,
    task:            Option<JoinHandle<()>>,
    runtime:         Option<Handle>
}

impl Weather {
    pub fn new(
        location: String,
        api_key: Option<String>,
        use_celsius: bool,
        update_interval_minutes: u64
    ) -> Self {
        Self {
            data: WeatherData::new(location, use_celsius),
            api_key,
            update_interval: Duration::from_secs(
                update_interval_minutes.clamp(1, 24 * 60).saturating_mul(60)
            ),
            sender: None,
            task: None,
            runtime: None
        }
    }

    /// Get current weather data for rendering
    pub fn data(&self) -> &WeatherData {
        &self.data
    }

    /// Exposes the clamped refresh period so tests can verify the clamping
    /// without reaching into private state.
    #[cfg(test)]
    fn update_interval(&self) -> Duration {
        self.update_interval
    }

    /// Restates the module for a configuration that may have changed.
    ///
    /// The module was built from the configuration at startup; without this a
    /// corrected API key or a new city would be ignored until the bar is
    /// restarted. The reading is kept when nothing changed, and reset when the
    /// place or the unit did — the old value would be an answer to another
    /// question.
    pub fn configure(
        &mut self,
        location: String,
        api_key: Option<String>,
        use_celsius: bool,
        update_interval_minutes: u64
    ) {
        let unchanged = self.data.location == location
            && self.api_key == api_key
            && self.data.use_celsius == use_celsius;

        self.api_key = api_key;
        self.update_interval =
            Duration::from_secs(update_interval_minutes.clamp(1, 24 * 60).saturating_mul(60));

        if !unchanged {
            self.data = WeatherData::new(location, use_celsius);
        }
    }

    /// Starts the refresh loop on the runtime the context carries.
    ///
    /// The loop owns the first fetch too: a tokio interval yields its first
    /// tick immediately, so the reading appears as soon as the task starts,
    /// and there is no second, untracked task to leak when the module stops.
    /// The runtime handle is kept so a manual refresh can spawn on the same
    /// runtime instead of assuming one is ambient.
    pub fn register(&mut self, ctx: &ModuleContext) {
        self.sender = Some(ctx.module_sender(|event: WeatherEvent| match event {
            WeatherEvent::Updated(data) => ModuleEvent::Weather(Message::Update(data)),
            WeatherEvent::Error(err) => ModuleEvent::Weather(Message::Error(err))
        }));
        self.runtime = Some(ctx.runtime_handle().clone());

        if let Some(task) = self.task.take() {
            task.abort();
        }

        if self.api_key.is_none() {
            warn!(
                "Weather module: no API key configured in config.toml, \
                 weather data will not be available"
            );
            self.data.description = String::from("No API key");
            return;
        }

        if let Some(sender) = self.sender.clone() {
            let interval_duration = self.update_interval;
            let location = self.data.location.clone();
            let use_celsius = self.data.use_celsius;
            let api_key = self.api_key.clone();

            self.task = Some(ctx.runtime_handle().spawn(async move {
                let mut ticker = interval(interval_duration);

                loop {
                    ticker.tick().await;

                    match Self::fetch_weather(&location, &api_key).await {
                        Ok(response) => {
                            let data = WeatherData::from_response(
                                response,
                                location.clone(),
                                use_celsius
                            );
                            if let Err(err) = sender.try_send(WeatherEvent::Updated(data)) {
                                error!("Failed to publish weather update: {err}");
                            }
                        }
                        Err(err) => {
                            error!("Failed to fetch weather: {err}");
                            if let Err(e) = sender.try_send(WeatherEvent::Error(err.to_string())) {
                                error!("Failed to publish weather error: {e}");
                            }
                        }
                    }
                }
            }));
        }
    }

    /// Aborts the refresh loop, keeping the last reading in place.
    ///
    /// The loop issues a network request per tick, so a bar whose clock does
    /// not display weather must not keep one running: nothing would ever read
    /// the answer.
    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
        self.runtime = None;
    }

    /// Update weather state from GUI message
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update(data) => {
                self.data = data;
            }
            Message::Error(err) => {
                error!("Weather module error: {err}");
                self.data.description = format!("Error: {err}");
            }
            Message::Refresh => {
                if let (Some(sender), Some(runtime)) = (&self.sender, &self.runtime) {
                    let location = self.data.location.clone();
                    let use_celsius = self.data.use_celsius;
                    let api_key = self.api_key.clone();
                    let update_sender = sender.clone();

                    runtime.spawn(async move {
                        match Self::fetch_weather(&location, &api_key).await {
                            Ok(response) => {
                                let data =
                                    WeatherData::from_response(response, location, use_celsius);
                                let _ = update_sender.try_send(WeatherEvent::Updated(data));
                            }
                            Err(err) => {
                                let _ =
                                    update_sender.try_send(WeatherEvent::Error(err.to_string()));
                            }
                        }
                    });
                }
            }
        }
    }

    /// Fetch weather data from OpenWeatherMap API
    async fn fetch_weather(
        location: &str,
        api_key: &Option<String>
    ) -> AppResult<WeatherResponse> {
        let api_key = api_key
            .as_ref()
            .ok_or_else(|| AppError::internal("Weather API key not configured in config.toml"))?;

        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}",
            location, api_key
        );

        let response = crate::utils::http_client().get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                AppError::internal(format!("Weather API timeout for location '{}'", location))
            } else if e.is_connect() {
                AppError::internal("No internet connection - cannot fetch weather")
            } else {
                AppError::internal(format!("Network error fetching weather: {}", e))
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::internal(match status.as_u16() {
                401 => format!("Invalid weather API key ({})", status),
                404 => format!("Location '{}' not found in weather database", location),
                429 => "Weather API rate limit exceeded - try again later".to_string(),
                500..=599 => format!("Weather API server error ({})", status),
                _ => format!(
                    "Weather API returned error {} for location '{}'",
                    status, location
                )
            }));
        }

        let weather = response.json::<WeatherResponse>().await.map_err(|e| {
            AppError::internal(format!("Invalid weather data format from API: {}", e))
        })?;

        Ok(weather)
    }
}

impl<M> Module<M> for Weather
where
    M: 'static + Clone
{
    type RegistrationData<'a> = &'a WeatherModuleConfig;
    type ViewData<'a> = ();

    /// Restates the module for the given configuration and starts its
    /// refresh loop.
    ///
    /// Weather has no bar section of its own — the clock hosts the readout —
    /// so registration is the whole contract: adopt whatever the configuration
    /// says now, then keep the reading fresh. Folding `configure` in here is
    /// what lets the bar treat weather like every other module instead of
    /// hand-wiring it.
    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.configure(
            config.location.clone(),
            config.api_key.clone(),
            config.use_celsius,
            config.update_interval_minutes
        );
        self.register(ctx);
        Ok(())
    }

    /// Stops the refresh loop once nothing on the bar shows weather.
    fn deregister(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_zero_interval_is_clamped_up_to_one_minute() {
        let weather = Weather::new(String::from("London"), None, true, 0);
        assert_eq!(weather.update_interval(), Duration::from_secs(60));
    }

    #[test]
    fn a_huge_interval_does_not_panic_and_caps_at_twenty_four_hours() {
        let weather = Weather::new(String::from("London"), None, true, u64::MAX);
        assert_eq!(weather.update_interval(), Duration::from_secs(24 * 60 * 60));
    }

    #[test]
    fn configure_keeps_the_reading_when_nothing_relevant_changed() {
        let mut weather =
            Weather::new(String::from("London"), Some(String::from("key")), true, 10);
        weather.update(Message::Update(WeatherData::from_response(
            freezing_point_response(),
            String::from("London"),
            true
        )));

        weather.configure(String::from("London"), Some(String::from("key")), true, 30);

        assert_eq!(weather.data().temperature, "0°C");
        assert_eq!(weather.data().description, "clear sky");
        assert_eq!(weather.update_interval(), Duration::from_secs(30 * 60));
    }

    #[test]
    fn configure_resets_the_reading_when_the_location_changes() {
        let mut weather =
            Weather::new(String::from("London"), Some(String::from("key")), true, 10);
        weather.update(Message::Update(WeatherData::from_response(
            freezing_point_response(),
            String::from("London"),
            true
        )));

        weather.configure(String::from("Paris"), Some(String::from("key")), true, 10);

        assert_eq!(weather.data().location, "Paris");
        assert_eq!(weather.data().temperature, "--");
        assert_eq!(weather.data().description, "Loading...");
    }

    #[test]
    fn from_response_converts_the_freezing_point_to_zero_celsius() {
        let data =
            WeatherData::from_response(freezing_point_response(), String::from("London"), true);
        assert_eq!(data.temperature, "0°C");
        assert_eq!(data.humidity, "50%");
        assert_eq!(data.wind_speed, "1.5 m/s");
    }

    #[test]
    fn from_response_converts_the_freezing_point_to_thirty_two_fahrenheit() {
        let data =
            WeatherData::from_response(freezing_point_response(), String::from("London"), false);
        assert_eq!(data.temperature, "32°F");
        assert!(!data.use_celsius);
    }
}
