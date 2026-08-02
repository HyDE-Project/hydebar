//! The calendar: a month of days, walked one month at a time.
//!
//! A building block of its own, not a bolt-on of the clock: the clock tells
//! the time, this module knows what a month looks like. The bar entry that
//! opens it lives with the clock, which is exactly the composition — a widget
//! made of two blocks, each blind to the other's internals.
//!
//! One folder, three rooms: [`month`] knows what a month is, [`metrics`]
//! knows how much room the view needs, [`view`] draws it. The module root
//! holds the state and the messages, and is all the outside ever talks to.

use iced::Element;
pub use month::{CalendarData, CalendarError, CalendarState, DayInfo};

use crate::components::icons::IconTheme;

mod metrics;
mod month;
mod view;

/// What the user asks of the calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    PreviousMonth,
    NextMonth,
    /// Return to the month that holds today.
    Today
}

/// The calendar module: the month in view and nothing else.
#[derive(Debug, Default)]
pub struct Calendar {
    state: CalendarState
}

impl Calendar {
    /// Month and year currently in view.
    #[must_use]
    pub const fn state(&self) -> &CalendarState {
        &self.state
    }

    /// Width the menu box needs: the grid, its own padding, the box padding.
    ///
    /// Stated by the module so the box hugs the grid; a stock menu width
    /// leaves a blank margin beside a grid that cannot grow into it.
    #[must_use]
    pub fn content_width(font_size: f32) -> f32 {
        metrics::content_width(font_size)
    }

    /// Height the menu content needs, from the same constants the view uses.
    #[must_use]
    pub fn content_height() -> f32 {
        metrics::content_height()
    }

    /// Applies what the user asked.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::PreviousMonth => self.state.previous_month(),
            Message::NextMonth => self.state.next_month(),
            Message::Today => self.state = CalendarState::current()
        }
    }

    /// The month view: navigation, weekday header and the day grid.
    #[must_use]
    pub fn menu_view(&self, icons: &IconTheme) -> Element<'_, Message> {
        view::month_view(&self.state, icons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_returns_the_view_to_the_current_month() {
        let mut calendar = Calendar::default();

        calendar.update(Message::NextMonth);
        calendar.update(Message::NextMonth);
        calendar.update(Message::Today);

        assert_eq!(calendar.state(), &CalendarState::current());
    }

    #[test]
    fn the_box_is_wider_than_the_grid_it_wraps() {
        assert!(Calendar::content_width(10.0) > 0.0);
        assert!(Calendar::content_height() > 0.0);
    }
}
