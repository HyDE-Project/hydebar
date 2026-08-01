//! The clock: the current time in the configured formats, and nothing else.
//!
//! The calendar and the weather are modules of their own; this one only knows
//! what time it is. Its bar entry opens the calendar menu, which is
//! composition between blocks, not knowledge of each other's insides.

use std::time::Duration;

use chrono::{DateTime, Local};
use iced::Element;
use tokio::{task::JoinHandle, time::sleep};

use crate::{
    ModuleContext, ModuleEventSender,
    components::text::text,
    config::ClockModuleConfig,
    event_bus::ModuleEvent,
    format_cycle::FormatCycle,
    menu::MenuType,
    modules::{Module, ModuleError, OnModulePress}
};

/// The moment the clock last read.
#[derive(Debug, Clone)]
pub struct ClockData {
    pub current_time: DateTime<Local>
}

impl ClockData {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_time: Local::now()
        }
    }

    pub fn update(&mut self) {
        self.current_time = Local::now();
    }

    /// The time rendered through a `chrono` format string.
    #[must_use]
    pub fn format(&self, format: &str) -> String {
        self.current_time.format(format).to_string()
    }
}

impl Default for ClockData {
    fn default() -> Self {
        Self::new()
    }
}

/// Events emitted by the clock module.
#[derive(Debug, Clone)]
pub enum ClockEvent {
    Tick(DateTime<Local>)
}

/// What the clock reacts to.
#[derive(Debug, Clone, Copy)]
pub enum Message {
    Update,
    /// Switch to the next configured format, wrapping after the last one.
    NextFormat
}

/// Renders `now` through every format the module can be switched to.
///
/// The tick loop compares these strings instead of the timestamp: a bar showing
/// `%H:%M` gains nothing from waking eleven times a minute, and the whole
/// window repaints on every event the module publishes. Rendering every
/// configured format, not just the active one, keeps the comparison correct
/// across a press that cycles to an alternative.
fn render_all(formats: &[String], now: DateTime<Local>) -> Vec<String> {
    formats
        .iter()
        .map(|format| now.format(format).to_string())
        .collect()
}

/// Time left until the next multiple of `period` on the wall clock.
///
/// A plain `interval` starts counting from whenever the bar happened to launch,
/// so a minute clock started at `12:00:30` would flip its display half a minute
/// late for the rest of the session. Sleeping to the boundary instead keeps the
/// displayed minute correct while still costing exactly one wakeup per period.
///
/// Landing exactly on a boundary yields a whole period rather than zero, so the
/// caller never spins publishing the same value twice.
fn duration_until_next_tick(now: DateTime<Local>, period: Duration) -> Duration {
    let period_nanos = i128::try_from(period.as_nanos()).unwrap_or(i128::MAX);

    if period_nanos <= 0 {
        return Duration::ZERO;
    }

    let elapsed =
        i128::from(now.timestamp()) * 1_000_000_000 + i128::from(now.timestamp_subsec_nanos());
    let remaining = period_nanos - elapsed.rem_euclid(period_nanos);

    Duration::from_nanos(u64::try_from(remaining).unwrap_or(u64::MAX))
}

/// The clock module.
#[derive(Debug)]
pub struct Clock {
    data:          ClockData,
    tick_interval: Duration,
    sender:        Option<ModuleEventSender<ClockEvent>>,
    task:          Option<JoinHandle<()>>,
    format:        FormatCycle,
    shown:         crate::components::crossfade::Crossfade
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            data:          ClockData::new(),
            tick_interval: Duration::from_secs(5),
            sender:        None,
            task:          None,
            format:        FormatCycle::new(),
            shown:         crate::components::crossfade::Crossfade::default()
        }
    }
}

impl Clock {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The moment the clock last read.
    #[must_use]
    pub const fn data(&self) -> &ClockData {
        &self.data
    }

    /// Format string the active index selects.
    #[must_use]
    pub fn active_format<'a>(&self, config: &'a ClockModuleConfig) -> &'a str {
        self.format.resolve(&config.format, &config.format_alt)
    }

    /// Starts the tick loop for the configured formats.
    pub fn register(&mut self, ctx: &ModuleContext, config: &ClockModuleConfig) {
        self.tick_interval = Self::determine_interval(config);
        self.data.update();
        self.sender =
            Some(ctx.module_sender(|_event: ClockEvent| ModuleEvent::Clock(Message::Update)));

        if let Some(task) = self.task.take() {
            task.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let period = self.tick_interval;
            let update_sender = sender;
            let formats: Vec<String> = config.formats().map(str::to_owned).collect();

            self.task = Some(ctx.runtime_handle().spawn(async move {
                let mut rendered: Option<Vec<String>> = None;

                loop {
                    sleep(duration_until_next_tick(Local::now(), period)).await;

                    let now = Local::now();
                    let next = render_all(&formats, now);

                    if rendered.as_ref() == Some(&next) {
                        continue;
                    }

                    rendered = Some(next);

                    update_sender.send(ClockEvent::Tick(now));
                }
            }));
        }
    }

    /// Aborts the tick loop, leaving the last rendered time in place.
    ///
    /// Registration is the only way back, so the bar can park the clock while
    /// the layout does not show it without dropping the module state.
    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }

    /// Applies what the user or the tick loop asked.
    ///
    /// `animated` decides whether the rendered time dissolves into its
    /// replacement or swaps outright.
    pub fn update(&mut self, message: Message, config: &ClockModuleConfig, animated: bool) {
        match message {
            Message::Update => {
                self.data.update();
            }
            Message::NextFormat => {
                self.format.advance(&config.format_alt);
            }
        }

        self.shown
            .set(self.data.format(self.active_format(config)), animated);
    }

    /// Advances the dissolve of the rendered time.
    pub fn tick_fade(&mut self, elapsed: Duration) -> bool {
        self.shown.advance(elapsed)
    }

    /// Whether the rendered time is still dissolving.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.shown.is_animating()
    }

    /// Determine tick interval from every format the module can render.
    ///
    /// A clock showing seconds has to wake once per second; one showing only
    /// minutes gains nothing from waking more often than once per minute, and
    /// every extra wakeup repaints the whole bar. The fastest format wins so
    /// switching to an alternative never leaves the clock ticking too slowly
    /// for the seconds it displays.
    fn determine_interval(config: &ClockModuleConfig) -> Duration {
        if config.formats().any(Self::format_shows_seconds) {
            Duration::from_secs(1)
        } else {
            Duration::from_mins(1)
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

    /// Stops the tick loop once the clock leaves the bar.
    ///
    /// A tick repaints every surface the bar owns, which is pure waste when no
    /// section renders the time any more.
    fn deregister(&mut self) {
        self.stop();
    }

    /// Renders the clock in its active format.
    ///
    /// A clock declaring alternatives cycles them on the left button and moves
    /// the calendar to the right button, the way waybar binds `format-alt`.
    fn view(
        &self,
        config: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let clock_text = if self.shown.current().is_empty() {
            text(self.data.format(self.active_format(config))).into()
        } else {
            self.shown.element(crate::components::scale::base())
        };
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
            format_alt:   alternatives.iter().map(ToString::to_string).collect(),
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
        assert_eq!(interval, Duration::from_mins(1));
    }

    fn at(hour: u32, minute: u32, second: u32, nanos: u32) -> DateTime<Local> {
        use chrono::TimeZone;

        Local
            .with_ymd_and_hms(2026, 7, 29, hour, minute, second)
            .single()
            .expect("unambiguous local time")
            + chrono::Duration::nanoseconds(i64::from(nanos))
    }

    #[test]
    fn a_minute_clock_sleeps_only_to_the_next_minute_boundary() {
        let delay = duration_until_next_tick(at(12, 34, 30, 0), Duration::from_mins(1));

        assert_eq!(delay, Duration::from_secs(30));
    }

    #[test]
    fn a_second_clock_sleeps_only_to_the_next_second_boundary() {
        let delay = duration_until_next_tick(at(12, 34, 30, 250_000_000), Duration::from_secs(1));

        assert_eq!(delay, Duration::from_millis(750));
    }

    #[test]
    fn landing_on_a_boundary_waits_a_whole_period_instead_of_spinning() {
        let delay = duration_until_next_tick(at(12, 34, 0, 0), Duration::from_mins(1));

        assert_eq!(delay, Duration::from_mins(1));
    }

    #[test]
    fn a_minute_clock_never_sleeps_longer_than_a_minute() {
        for second in 0..60 {
            let delay = duration_until_next_tick(at(12, 34, second, 0), Duration::from_mins(1));

            assert!(delay <= Duration::from_mins(1));
            assert!(delay > Duration::ZERO);
        }
    }

    #[test]
    fn a_minute_that_did_not_change_publishes_nothing() {
        let formats = vec!["%H:%M".to_string()];
        let first = render_all(&formats, at(12, 34, 0, 0));
        let same_minute = render_all(&formats, at(12, 34, 59, 0));
        let next_minute = render_all(&formats, at(12, 35, 0, 0));

        assert_eq!(first, same_minute);
        assert_ne!(first, next_minute);
    }

    #[test]
    fn an_alternative_format_is_compared_alongside_the_active_one() {
        let formats = vec!["%H:%M".to_string(), "%S".to_string()];

        let first = render_all(&formats, at(12, 34, 10, 0));
        let later = render_all(&formats, at(12, 34, 11, 0));

        assert_ne!(first, later);
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

        clock.update(Message::NextFormat, &config, false);
        assert_eq!(clock.active_format(&config), "%d.%m.%y");

        clock.update(Message::NextFormat, &config, false);
        assert_eq!(clock.active_format(&config), "%A");

        clock.update(Message::NextFormat, &config, false);
        assert_eq!(clock.active_format(&config), "%H:%M");
    }

    #[test]
    fn a_clock_without_alternatives_keeps_its_format() {
        let config = config("%H:%M", &[]);
        let mut clock = Clock::new();

        clock.update(Message::NextFormat, &config, false);

        assert_eq!(clock.active_format(&config), "%H:%M");
    }

    #[test]
    fn the_rendered_text_follows_the_active_format() {
        let config = config("%H", &["%M"]);
        let mut clock = Clock::new();

        let hours = clock.data().format(clock.active_format(&config));
        clock.update(Message::NextFormat, &config, false);
        let minutes = clock.data().format(clock.active_format(&config));

        assert_eq!(hours, clock.data().current_time.format("%H").to_string());
        assert_eq!(minutes, clock.data().current_time.format("%M").to_string());
    }
}
