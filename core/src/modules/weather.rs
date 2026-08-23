//! The weather: one reading, refreshed on a clock, shown where asked.
//!
//! One folder, four rooms: [`data`] says what a reading is, [`fetch`] makes
//! the round-trip to the API, [`state`] runs the refresh loop and folds
//! messages in, [`module`] draws the bar entry and wires the module to the
//! bar. The root holds the state the rooms share.

use std::time::Duration;

use tokio::{runtime::Handle, task::JoinHandle};

use crate::ModuleEventSender;

mod data;
mod fetch;
mod module;
mod state;

pub use data::{
    MainWeather, Message, WeatherCondition, WeatherData, WeatherEvent, WeatherResponse, Wind
};

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
    /// A weather entry that has not read anything yet.
    #[must_use]
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
    #[must_use]
    pub const fn data(&self) -> &WeatherData {
        &self.data
    }

    /// Exposes the clamped refresh period so tests can verify the clamping
    /// without reaching into private state.
    #[cfg(test)]
    const fn update_interval(&self) -> Duration {
        self.update_interval
    }
}
