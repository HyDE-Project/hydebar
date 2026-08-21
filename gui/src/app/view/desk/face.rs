//! The clock's own face, at the size the canvas has room for.
//!
//! The strip has room for one line, so the clock spends it on whatever the
//! configured format asks for. The canvas has room for the hour to be read
//! from across the room and the date under it, with the month standing open
//! below — which is what the clock has always been, only folded.

use hydebar_core::modules::clock::ClockData;
use hydebar_proto::config::ClockModuleConfig;
use iced::{
    Alignment, Element, Length,
    widget::{Column, text}
};

use crate::app::Message;

/// The two formats the face is drawn in.
///
/// The canvas has room for one line of hour and one of date, which is not
/// what the strip's format says, so it cannot simply be borrowed. What it can
/// be asked is whether this session reads its clock in twelve hours or in
/// twenty-four, and that answer carries over.
const fn formats(config: &ClockModuleConfig) -> (&'static str, &'static str) {
    (hour_format(config), "%A, %-d %B")
}

/// The hour format matching the way the strip states its own clock.
const fn hour_format(config: &ClockModuleConfig) -> &'static str {
    if twelve_hour(config.format.as_bytes(), 0) {
        "%I:%M"
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
    size: f32,
    align: Alignment
) -> Element<'a, Message> {
    let (time, day) = formats(config);

    Column::new()
        .push(text(data.format(time)).size(size * 3.4))
        .push(text(data.format(day)).size(size * 1.1))
        .spacing(size * 0.1)
        .width(Length::Fill)
        .align_x(align)
        .into()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_session_reading_twenty_four_hours_keeps_them_on_the_canvas() {
        assert_eq!(hour_format(&ClockModuleConfig::default()), "%H:%M");
    }

    #[test]
    fn a_session_reading_twelve_hours_carries_that_over() {
        for format in ["%I:%M %p", "%a %d %b %I:%M", "%p"] {
            let config = ClockModuleConfig {
                format: format.to_owned(),
                ..ClockModuleConfig::default()
            };

            assert_eq!(hour_format(&config), "%I:%M", "format {format}");
        }
    }

    #[test]
    fn the_date_line_never_repeats_the_hour() {
        let (time, day) = formats(&ClockModuleConfig::default());

        assert_eq!(time, "%H:%M");
        assert!(!day.contains("%H"));
        assert!(!day.contains("%M"));
    }
}
