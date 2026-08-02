//! Message folding and the format cycle for the clock module.

use std::time::Duration;

use super::{Clock, Message};
use crate::config::ClockModuleConfig;

impl Clock {
    /// Format string the active index selects.
    #[must_use]
    pub fn active_format<'a>(&self, config: &'a ClockModuleConfig) -> &'a str {
        self.format.resolve(&config.format, &config.format_alt)
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
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn config(format: &str, alternatives: &[&str]) -> ClockModuleConfig {
        ClockModuleConfig {
            format:       format.to_string(),
            format_alt:   alternatives.iter().map(ToString::to_string).collect(),
            show_weather: false
        }
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
