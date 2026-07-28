mod calendar;
mod view;

use std::time::Duration;

pub use calendar::{CalendarData, CalendarError, CalendarState, DayInfo};
use chrono::{DateTime, Local};
use iced::Element;
use log::error;
use tokio::{task::JoinHandle, time::interval};

use crate::{
    ModuleContext, ModuleEventSender,
    components::icons::IconTheme,
    config::ClockModuleConfig,
    event_bus::ModuleEvent,
    format_cycle::FormatCycle,
    menu::MenuType,
    modules::{Module, ModuleError, OnModulePress, weather::WeatherData}
};

/// Clock data for rendering
#[derive(Debug, Clone)]
pub struct ClockData {
    pub current_time: DateTime<Local>,
    pub weather:      Option<WeatherData>
}

impl ClockData {
    pub fn new() -> Self {
        Self {
            current_time: Local::now(),
            weather:      None
        }
    }

    pub fn update(&mut self) {
        self.current_time = Local::now();
    }

    pub fn update_weather(&mut self, weather: WeatherData) {
        self.weather = Some(weather);
    }

    /// Format the time according to chrono format string
    pub fn format(&self, format: &str) -> String {
        self.current_time.format(format).to_string()
    }
}

impl Default for ClockData {
    fn default() -> Self {
        Self::new()
    }
}

/// Events emitted by the clock module
#[derive(Debug, Clone)]
pub enum ClockEvent {
    Tick(DateTime<Local>)
}

/// Message type for GUI communication
#[derive(Debug, Clone)]
pub enum Message {
    Update,
    UpdateWeather(WeatherData),
    PreviousMonth,
    NextMonth,
    /// Switch to the next configured format, wrapping after the last one.
    NextFormat
}

/// Clock module - business logic only, no GUI!
#[derive(Debug)]
pub struct Clock {
    data:           ClockData,
    tick_interval:  Duration,
    sender:         Option<ModuleEventSender<ClockEvent>>,
    task:           Option<JoinHandle<()>>,
    calendar_state: CalendarState,
    format:         FormatCycle
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            data:           ClockData::new(),
            tick_interval:  Duration::from_secs(5),
            sender:         None,
            task:           None,
            calendar_state: CalendarState::default(),
            format:         FormatCycle::new()
        }
    }
}

impl Clock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get current clock data for rendering
    pub fn data(&self) -> &ClockData {
        &self.data
    }

    /// Get current calendar state for rendering
    pub fn calendar_state(&self) -> &CalendarState {
        &self.calendar_state
    }

    /// Format string the active index selects.
    pub fn active_format<'a>(&self, config: &'a ClockModuleConfig) -> &'a str {
        self.format.resolve(&config.format, &config.format_alt)
    }

    /// Initialize with module context and clock configuration
    pub fn register(&mut self, ctx: &ModuleContext, config: &ClockModuleConfig) {
        self.tick_interval = Self::determine_interval(config);
        self.data.update();
        self.sender =
            Some(ctx.module_sender(|_event: ClockEvent| ModuleEvent::Clock(Message::Update)));

        if let Some(task) = self.task.take() {
            task.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let interval_duration = self.tick_interval;
            let update_sender = sender.clone();

            self.task = Some(ctx.runtime_handle().spawn(async move {
                let mut ticker = interval(interval_duration);

                loop {
                    ticker.tick().await;
                    let now = Local::now();

                    if let Err(err) = update_sender.try_send(ClockEvent::Tick(now)) {
                        error!("Failed to publish clock tick: {err}");
                    }
                }
            }));
        }
    }

    /// Update clock state from GUI message
    pub fn update(&mut self, message: Message, config: &ClockModuleConfig) {
        match message {
            Message::Update => {
                self.data.update();
            }
            Message::UpdateWeather(weather) => {
                self.data.update_weather(weather);
            }
            Message::PreviousMonth => {
                self.calendar_state.previous_month();
            }
            Message::NextMonth => {
                self.calendar_state.next_month();
            }
            Message::NextFormat => {
                self.format.advance(&config.format_alt);
            }
        }
    }

    /// Renders the calendar menu view.
    pub fn menu_view(&self, icons: &IconTheme) -> Element<'_, Message> {
        view::build_calendar_menu_view(&self.calendar_state, icons)
    }

    /// Determine tick interval from every format the module can render.
    ///
    /// The fastest format wins so switching to an alternative never leaves the
    /// clock ticking too slowly for the seconds it displays.
    fn determine_interval(config: &ClockModuleConfig) -> Duration {
        if config.formats().any(Self::format_shows_seconds) {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        }
    }

    /// Reports whether a `chrono` format string renders seconds.
    fn format_shows_seconds(format: &str) -> bool {
        const SECOND_SPECIFIERS: [&str; 6] = ["%S", "%T", "%X", "%r", "%:z", "%s"];

        SECOND_SPECIFIERS
            .iter()
            .any(|specifier| format.contains(specifier))
    }
}

impl<M> Module<M> for Clock
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = &'a ClockModuleConfig;
    type RegistrationData<'a> = &'a ClockModuleConfig;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.register(ctx, config);
        Ok(())
    }

    /// Renders the clock in its active format.
    ///
    /// A clock declaring alternatives cycles them on the left button and moves
    /// the calendar to the right button, the way waybar binds `format-alt`.
    fn view(
        &self,
        config: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        use iced::widget::text;

        let clock_text = text(self.data.format(self.active_format(config))).into();
        let on_press = if config.has_alternatives() {
            OnModulePress::Action(Box::new(M::from(Message::NextFormat)))
        } else {
            OnModulePress::ToggleMenu(MenuType::Calendar)
        };

        Some((clock_text, Some(on_press)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_data_format() {
        let data = ClockData::new();
        let formatted = data.format("%H:%M");
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 5);
    }

    fn config(format: &str, alternatives: &[&str]) -> ClockModuleConfig {
        ClockModuleConfig {
            format:       format.to_string(),
            format_alt:   alternatives.iter().map(|alt| alt.to_string()).collect(),
            show_weather: false
        }
    }

    #[test]
    fn determine_interval_with_seconds() {
        let interval = Clock::determine_interval(&config("%H:%M:%S", &[]));
        assert_eq!(interval, Duration::from_secs(1));
    }

    #[test]
    fn determine_interval_without_seconds() {
        let interval = Clock::determine_interval(&config("%H:%M", &[]));
        assert_eq!(interval, Duration::from_secs(5));
    }

    #[test]
    fn determine_interval_follows_the_fastest_alternative() {
        let interval = Clock::determine_interval(&config("%H:%M", &["%H:%M:%S"]));
        assert_eq!(interval, Duration::from_secs(1));
    }

    #[test]
    fn a_press_walks_the_configured_formats_and_wraps_around() {
        let config = config("%H:%M", &["%d.%m.%y", "%A"]);
        let mut clock = Clock::new();

        assert_eq!(clock.active_format(&config), "%H:%M");

        clock.update(Message::NextFormat, &config);
        assert_eq!(clock.active_format(&config), "%d.%m.%y");

        clock.update(Message::NextFormat, &config);
        assert_eq!(clock.active_format(&config), "%A");

        clock.update(Message::NextFormat, &config);
        assert_eq!(clock.active_format(&config), "%H:%M");
    }

    #[test]
    fn a_clock_without_alternatives_keeps_its_format() {
        let config = config("%H:%M", &[]);
        let mut clock = Clock::new();

        clock.update(Message::NextFormat, &config);

        assert_eq!(clock.active_format(&config), "%H:%M");
    }

    #[test]
    fn the_rendered_text_follows_the_active_format() {
        let config = config("%H", &["%M"]);
        let mut clock = Clock::new();

        let hours = clock.data().format(clock.active_format(&config));
        clock.update(Message::NextFormat, &config);
        let minutes = clock.data().format(clock.active_format(&config));

        assert_eq!(hours, clock.data().current_time.format("%H").to_string());
        assert_eq!(minutes, clock.data().current_time.format("%M").to_string());
    }
}
