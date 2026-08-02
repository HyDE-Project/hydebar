//! How the module reacts: configuration, the refresh loop, message folding.

use std::time::Duration;

use log::{error, warn};
use tokio::time::interval;

use super::{
    Weather,
    data::{Message, WeatherData, WeatherEvent},
    fetch::fetch_weather
};
use crate::{ModuleContext, event_bus::ModuleEvent};

impl Weather {
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

                    match fetch_weather(&location, api_key.as_ref()).await {
                        Ok(response) => {
                            let data =
                                WeatherData::from_response(&response, location.clone(), use_celsius);
                            sender.send(WeatherEvent::Updated(data));
                        }
                        Err(err) => {
                            error!("Failed to fetch weather: {err}");
                            sender.send(WeatherEvent::Error(err.to_string()));
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
                        match fetch_weather(&location, api_key.as_ref()).await {
                            Ok(response) => {
                                let data =
                                    WeatherData::from_response(&response, location, use_celsius);
                                update_sender.send(WeatherEvent::Updated(data));
                            }
                            Err(err) => {
                                update_sender.send(WeatherEvent::Error(err.to_string()));
                            }
                        }
                    });
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{
        super::data::{MainWeather, WeatherCondition, WeatherResponse, Wind},
        *
    };

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
        assert_eq!(weather.update_interval(), Duration::from_mins(1));
    }

    #[test]
    fn a_huge_interval_does_not_panic_and_caps_at_twenty_four_hours() {
        let weather = Weather::new(String::from("London"), None, true, u64::MAX);
        assert_eq!(weather.update_interval(), Duration::from_hours(24));
    }

    #[test]
    fn configure_keeps_the_reading_when_nothing_relevant_changed() {
        let mut weather =
            Weather::new(String::from("London"), Some(String::from("key")), true, 10);
        weather.update(Message::Update(WeatherData::from_response(
            &freezing_point_response(),
            String::from("London"),
            true
        )));

        weather.configure(String::from("London"), Some(String::from("key")), true, 30);

        assert_eq!(weather.data().temperature, "0°C");
        assert_eq!(weather.data().description, "clear sky");
        assert_eq!(weather.update_interval(), Duration::from_mins(30));
    }

    #[test]
    fn configure_resets_the_reading_when_the_location_changes() {
        let mut weather =
            Weather::new(String::from("London"), Some(String::from("key")), true, 10);
        weather.update(Message::Update(WeatherData::from_response(
            &freezing_point_response(),
            String::from("London"),
            true
        )));

        weather.configure(String::from("Paris"), Some(String::from("key")), true, 10);

        assert_eq!(weather.data().location, "Paris");
        assert_eq!(weather.data().temperature, "--");
        assert_eq!(weather.data().description, "Loading...");
    }
}
