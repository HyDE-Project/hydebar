//! The middle of the canvas: the hour at the size a room away can read it.

use hydebar_core::modules::{clock::ClockData, weather::WeatherData};
use hydebar_proto::config::ClockModuleConfig;
use iced::{
    Alignment, Element, Length,
    widget::{Column, text}
};

use super::blocks::Ink;
use crate::app::Message;

/// The two formats the hour is drawn in.
///
/// The canvas has room for one line of hour and one of date, which is not
/// what the strip's format says, so it cannot simply be borrowed. What it can
/// be asked is whether this session reads its clock in twelve hours or in
/// twenty-four, and that answer carries over.
const fn formats(config: &ClockModuleConfig) -> (&'static str, &'static str) {
    (hour_format(config), "%A, %-d %B %Y")
}

/// The hour format matching the way the strip states its own clock.
const fn hour_format(config: &ClockModuleConfig) -> &'static str {
    if twelve_hour(config.format.as_bytes(), 0) {
        "%I:%M %p"
    } else {
        "%H:%M"
    }
}

/// Reports whether a `chrono` format asks for a twelve hour clock.
///
/// Written as a scan rather than a search so it can be `const`: the two
/// specifiers that give a twelve hour reading are `%I` and `%p`.
const fn twelve_hour(format: &[u8], index: usize) -> bool {
    if index + 1 >= format.len() {
        return false;
    }

    if format[index] == b'%' && (format[index + 1] == b'I' || format[index + 1] == b'p') {
        return true;
    }

    twelve_hour(format, index + 1)
}

/// Draws the hour, with the date under it.
pub(super) fn clock<'a>(
    data: &ClockData,
    config: &ClockModuleConfig,
    ink: Ink
) -> Element<'a, Message> {
    let (time, day) = formats(config);

    Column::new()
        .push(
            text(data.format(time))
                .size(ink.size * 4.6)
                .color(ink.value)
        )
        .push(
            text(data.format(day))
                .size(ink.size * 1.15)
                .color(ink.value.scale_alpha(0.65))
        )
        .spacing(ink.size * 0.2)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

/// Draws the sky over the configured location.
pub(super) fn weather<'a>(data: &WeatherData, ink: Ink) -> Element<'a, Message> {
    Column::new()
        .push(
            text(data.temperature.clone())
                .size(ink.size * 2.0)
                .color(ink.value)
        )
        .push(
            text(data.description.clone())
                .size(ink.size * 1.1)
                .color(ink.value.scale_alpha(0.75))
        )
        .push(
            text(format!(
                "{} · humidity {} · wind {}",
                data.location, data.humidity, data.wind_speed
            ))
            .size(ink.size * 0.95)
            .color(ink.value.scale_alpha(0.55))
        )
        .spacing(ink.size * 0.2)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_session_reading_twenty_four_hours_keeps_them_on_the_canvas() {
        let config = ClockModuleConfig::default();

        assert_eq!(hour_format(&config), "%H:%M");
    }

    #[test]
    fn a_session_reading_twelve_hours_carries_that_over() {
        for format in ["%I:%M %p", "%a %d %b %I:%M", "%p"] {
            let config = ClockModuleConfig {
                format: format.to_owned(),
                ..ClockModuleConfig::default()
            };

            assert_eq!(hour_format(&config), "%I:%M %p", "format {format}");
        }
    }

    #[test]
    fn the_date_line_never_repeats_the_hour() {
        let (hour, date) = formats(&ClockModuleConfig::default());

        assert_eq!(hour, "%H:%M");
        assert!(!date.contains("%H"));
        assert!(!date.contains("%M"));
    }
}
