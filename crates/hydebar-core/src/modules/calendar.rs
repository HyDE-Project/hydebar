//! The calendar: a month of days, walked one month at a time.
//!
//! A building block of its own, not a bolt-on of the clock: the clock tells
//! the time, this module knows what a month looks like. The bar entry that
//! opens it lives with the clock, which is exactly the composition — a widget
//! made of two blocks, each blind to the other's internals.
//!
//! One file, three rooms: [`month`] knows what a month is, [`metrics`] knows
//! how much room the view needs, [`view`] draws it. The module root holds the
//! state and the messages, and is all the outside ever talks to.

use iced::Element;
pub use month::{CalendarData, CalendarError, CalendarState, DayInfo};

use crate::components::icons::IconTheme;

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
    pub fn state(&self) -> &CalendarState {
        &self.state
    }

    /// Width the menu box needs: the grid, its own padding, the box padding.
    ///
    /// Stated by the module so the box hugs the grid; a stock menu width
    /// leaves a blank margin beside a grid that cannot grow into it.
    pub fn content_width(font_size: f32) -> f32 {
        metrics::content_width(font_size)
    }

    /// Height the menu content needs, from the same constants the view uses.
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
    pub fn menu_view(&self, icons: &IconTheme) -> Element<'_, Message> {
        view::month_view(&self.state, icons)
    }
}

/// What a month is: days, shape and names. No rendering in this room.
mod month {
    use chrono::{Datelike, Local, Month, NaiveDate};

    /// Month and year the calendar is looking at.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CalendarState {
        year:  i32,
        month: u32
    }

    impl Default for CalendarState {
        fn default() -> Self {
            let now = Local::now();

            Self {
                year:  now.year(),
                month: now.month()
            }
        }
    }

    impl CalendarState {
        /// The month that holds today.
        pub fn current() -> Self {
            Self::default()
        }

        /// A specific month.
        ///
        /// # Errors
        ///
        /// Returns [`CalendarError::InvalidMonth`] when `month` is not 1–12.
        pub fn new(year: i32, month: u32) -> Result<Self, CalendarError> {
            if !(1..=12).contains(&month) {
                return Err(CalendarError::InvalidMonth {
                    month
                });
            }

            Ok(Self {
                year,
                month
            })
        }

        pub fn year(&self) -> i32 {
            self.year
        }

        /// Month in view, 1–12.
        pub fn month(&self) -> u32 {
            self.month
        }

        pub fn previous_month(&mut self) {
            if self.month == 1 {
                self.month = 12;
                self.year -= 1;
            } else {
                self.month -= 1;
            }
        }

        pub fn next_month(&mut self) {
            if self.month == 12 {
                self.month = 1;
                self.year += 1;
            } else {
                self.month += 1;
            }
        }

        /// English name of the month in view.
        pub fn month_name(&self) -> &'static str {
            Month::try_from(self.month as u8)
                .map(|month| month.name())
                .unwrap_or("Unknown")
        }

        /// The grid of days this month shows.
        pub fn generate_calendar(&self) -> CalendarData {
            CalendarData::generate(self.year, self.month)
        }
    }

    /// One day cell of the grid.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DayInfo {
        pub day:      u32,
        pub is_today: bool,
        pub in_month: bool
    }

    /// A month rendered as a fixed seven-by-six grid.
    ///
    /// The grid keeps its six rows whatever the month, so walking the months
    /// never changes the window height under the pointer.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CalendarData {
        pub days: Vec<DayInfo>
    }

    impl CalendarData {
        /// The grid for `year`/`month`, padded with the neighbouring months.
        pub fn generate(year: i32, month: u32) -> Self {
            let today = Local::now().date_naive();

            let first_day = NaiveDate::from_ymd_opt(year, month, 1)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, 1, 1).expect("fallback date"));
            let weekday = first_day.weekday().num_days_from_monday();
            let days_in_month = Self::days_in_month(year, month);
            let prev_month_days = if month == 1 {
                Self::days_in_month(year - 1, 12)
            } else {
                Self::days_in_month(year, month - 1)
            };

            let mut days = Vec::with_capacity(42);

            for i in 0..weekday {
                days.push(DayInfo {
                    day:      prev_month_days - weekday + i + 1,
                    is_today: false,
                    in_month: false
                });
            }

            for day in 1..=days_in_month {
                let date = NaiveDate::from_ymd_opt(year, month, day).unwrap_or(first_day);

                days.push(DayInfo {
                    day,
                    is_today: date == today,
                    in_month: true
                });
            }

            let remaining = 42 - days.len();

            for day in 1..=remaining {
                days.push(DayInfo {
                    day:      day as u32,
                    is_today: false,
                    in_month: false
                });
            }

            Self {
                days
            }
        }

        fn days_in_month(year: i32, month: u32) -> u32 {
            NaiveDate::from_ymd_opt(year, month, 1)
                .and_then(|date| {
                    if month == 12 {
                        NaiveDate::from_ymd_opt(year + 1, 1, 1)
                    } else {
                        NaiveDate::from_ymd_opt(year, month + 1, 1)
                    }
                    .map(|next| next.signed_duration_since(date).num_days() as u32)
                })
                .unwrap_or(30)
        }
    }

    /// What can go wrong when a month is named outright.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum CalendarError {
        /// Month value outside 1–12.
        InvalidMonth { month: u32 }
    }

    impl std::fmt::Display for CalendarError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                CalendarError::InvalidMonth {
                    month
                } => {
                    write!(f, "invalid month: {month}, must be in range 1-12")
                }
            }
        }
    }

    impl std::error::Error for CalendarError {}
}

/// Every size of the view, and the room the whole of it takes.
///
/// The window is sized from these same constants, so the box always hugs the
/// grid: a size changed here moves the drawing and the measurement together.
mod metrics {
    use crate::components::scale;

    /// Week starts on Monday, the way the reference desktop counts.
    pub(super) const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    /// Side of one day cell, in pixels of the reference theme.
    pub(super) const CELL: f32 = 36.0;

    /// Gap between day cells, in pixels of the reference theme.
    pub(super) const CELL_GAP: f32 = 4.0;

    /// Month title size, in pixels of the reference theme.
    pub(super) const TITLE_SIZE: f32 = 18.0;

    /// Weekday header size, in pixels of the reference theme.
    pub(super) const WEEKDAY_SIZE: f32 = 12.0;

    /// Day number size, in pixels of the reference theme.
    pub(super) const DAY_SIZE: f32 = 14.0;

    /// Gap between the header, the rule, the weekdays and the grid.
    pub(super) const SECTION_GAP: f32 = 8.0;

    /// Padding of the whole column, in pixels of the reference theme.
    pub(super) const OUTER_PADDING: f32 = 4.0;

    /// Vertical room the renderer's stock button padding adds to the header.
    const NAV_BUTTON_PADDING: f32 = 10.0;

    /// Height one line of text at `size` occupies at the stock line height.
    fn scaled_line(size: f32) -> f32 {
        scale::scaled(size) * 1.3
    }

    /// Width of the day grid alone.
    pub(super) fn grid_width() -> f32 {
        scale::scaled(7.0 * CELL + 6.0 * CELL_GAP)
    }

    /// Width the menu box needs, box padding included.
    pub(super) fn content_width(font_size: f32) -> f32 {
        scale::scaled(7.0 * CELL + 6.0 * CELL_GAP + 2.0 * OUTER_PADDING)
            + 2.0 * crate::menu::MENU_PADDING_EM * font_size
    }

    /// Height the menu content needs.
    pub(super) fn content_height() -> f32 {
        let header = scaled_line(TITLE_SIZE) + NAV_BUTTON_PADDING;
        let weekdays = scaled_line(WEEKDAY_SIZE);
        let grid = scale::scaled(6.0 * CELL + 5.0 * CELL_GAP);
        let rule = 1.0;
        let spacings = 3.0 * scale::scaled(SECTION_GAP);
        let padding = scale::scaled(2.0 * OUTER_PADDING);

        header + rule + weekdays + grid + spacings + padding
    }
}

/// Drawing of the month: header, weekday row, day grid and their styles.
mod view {
    use iced::{
        Alignment, Border, Color, Element, Length, Theme,
        widget::{Column, Row, button, column, container, row, rule}
    };

    use super::{
        Message,
        metrics::{
            CELL, CELL_GAP, DAY_SIZE, OUTER_PADDING, SECTION_GAP, TITLE_SIZE, WEEKDAY_SIZE,
            WEEKDAYS, grid_width
        },
        month::{CalendarState, DayInfo}
    };
    use crate::components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    };

    /// The whole month view.
    pub(super) fn month_view<'a>(
        state: &'a CalendarState,
        icons: &IconTheme
    ) -> Element<'a, Message> {
        let header = row![
            nav_button(icon(icons, Icons::LeftChevron), Message::PreviousMonth),
            button(
                container(
                    text(format!("{} {}", state.month_name(), state.year()))
                        .size(scale::scaled(TITLE_SIZE))
                )
                .width(Length::Fill)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .style(nav_button_style)
            .on_press(Message::Today),
            nav_button(icon(icons, Icons::RightChevron), Message::NextMonth),
        ]
        .align_y(Alignment::Center)
        .spacing(scale::scaled(SECTION_GAP));

        let weekday_header = Row::with_children(
            WEEKDAYS
                .iter()
                .map(|day| {
                    container(text(*day).size(scale::scaled(WEEKDAY_SIZE)))
                        .width(Length::Fixed(scale::scaled(CELL)))
                        .align_x(Alignment::Center)
                        .into()
                })
                .collect::<Vec<_>>()
        )
        .spacing(scale::scaled(CELL_GAP));

        let calendar_data = state.generate_calendar();
        let week_rows = calendar_data
            .days
            .chunks(7)
            .map(|week| {
                Row::with_children(week.iter().map(day_cell).collect::<Vec<_>>())
                    .spacing(scale::scaled(CELL_GAP))
                    .into()
            })
            .collect::<Vec<_>>();

        let calendar_grid = Column::with_children(week_rows).spacing(scale::scaled(CELL_GAP));

        column![header, rule::horizontal(1), weekday_header, calendar_grid]
            .spacing(scale::scaled(SECTION_GAP))
            .padding(scale::scaled(OUTER_PADDING))
            .width(Length::Fixed(grid_width()))
            .into()
    }

    /// One day of the grid as a cell button.
    fn day_cell<'a>(day_info: &DayInfo) -> Element<'a, Message> {
        let in_month = day_info.in_month;
        let is_today = day_info.is_today;

        button(
            container(text(day_info.day.to_string()).size(scale::scaled(DAY_SIZE)))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
        )
        .width(Length::Fixed(scale::scaled(CELL)))
        .height(Length::Fixed(scale::scaled(CELL)))
        .style(move |theme: &Theme, status: button::Status| {
            day_button_style(theme, status, in_month, is_today)
        })
        .into()
    }

    /// A chevron of the month navigation.
    fn nav_button<'a>(
        content: impl Into<Element<'a, Message>>,
        message: Message
    ) -> iced::widget::Button<'a, Message> {
        button(content.into())
            .on_press(message)
            .style(nav_button_style)
    }

    fn nav_button_style(theme: &Theme, status: button::Status) -> button::Style {
        let mut base = button::Style {
            background: None,
            border: Border {
                width:  0.0,
                radius: scale::scaled(4.0).into(),
                color:  Color::TRANSPARENT
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };

        if matches!(status, button::Status::Hovered) {
            base.background = Some(theme.extended_palette().background.weak.color.into());
        }

        base
    }

    /// Style of one day cell.
    ///
    /// Today is the one filled cell; every other day sits flat on the menu
    /// and lights up under the pointer. Days of the neighbouring months stay
    /// dimmed, so the shape of the month is readable at a glance.
    fn day_button_style(
        theme: &Theme,
        status: button::Status,
        in_month: bool,
        is_today: bool
    ) -> button::Style {
        let palette = theme.extended_palette();

        let (background, text_color) = if is_today {
            (
                Some(theme.palette().primary.into()),
                palette.primary.base.text
            )
        } else if in_month {
            (None, theme.palette().text)
        } else {
            (None, palette.background.weak.text)
        };

        let mut base = button::Style {
            background,
            border: Border {
                width:  0.0,
                radius: scale::scaled(8.0).into(),
                color:  Color::TRANSPARENT
            },
            text_color,
            ..button::Style::default()
        };

        if matches!(status, button::Status::Hovered) && !is_today {
            base.background = Some(palette.primary.weak.color.into());
            base.text_color = palette.primary.weak.text;
        }

        base
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Local};

    use super::{metrics::WEEKDAYS, *};

    #[test]
    fn the_default_view_is_the_current_month() {
        let state = CalendarState::default();
        let now = Local::now();

        assert_eq!(state.year(), now.year());
        assert_eq!(state.month(), now.month());
    }

    #[test]
    fn a_named_month_is_validated() {
        assert!(CalendarState::new(2024, 1).is_ok());
        assert!(CalendarState::new(2024, 12).is_ok());
        assert!(CalendarState::new(2024, 0).is_err());
        assert!(CalendarState::new(2024, 13).is_err());
    }

    #[test]
    fn walking_back_from_january_lands_in_december() {
        let mut state = CalendarState::new(2024, 1).expect("valid month");
        state.previous_month();

        assert_eq!(state.year(), 2023);
        assert_eq!(state.month(), 12);
    }

    #[test]
    fn walking_forward_from_december_lands_in_january() {
        let mut state = CalendarState::new(2024, 12).expect("valid month");
        state.next_month();

        assert_eq!(state.year(), 2025);
        assert_eq!(state.month(), 1);
    }

    #[test]
    fn months_walk_one_step_at_a_time() {
        let mut state = CalendarState::new(2024, 3).expect("valid month");

        state.previous_month();
        assert_eq!(state.month(), 2);

        state.next_month();
        state.next_month();
        assert_eq!(state.month(), 4);
        assert_eq!(state.year(), 2024);
    }

    #[test]
    fn months_carry_their_names() {
        assert_eq!(
            CalendarState::new(2024, 1).expect("valid").month_name(),
            "January"
        );
        assert_eq!(
            CalendarState::new(2024, 12).expect("valid").month_name(),
            "December"
        );
    }

    #[test]
    fn the_grid_always_holds_six_weeks() {
        assert_eq!(CalendarData::generate(2024, 10).days.len(), 42);
    }

    #[test]
    fn october_2024_starts_on_a_tuesday() {
        let data = CalendarData::generate(2024, 10);

        assert!(!data.days[0].in_month);
        assert!(data.days[1].in_month);
        assert_eq!(data.days[1].day, 1);
    }

    #[test]
    fn leap_and_plain_februaries_are_told_apart() {
        let leap = CalendarData::generate(2024, 2);
        let plain = CalendarData::generate(2023, 2);

        assert_eq!(leap.days.iter().filter(|d| d.in_month).count(), 29);
        assert_eq!(plain.days.iter().filter(|d| d.in_month).count(), 28);
    }

    #[test]
    fn today_returns_the_view_to_the_current_month() {
        let mut calendar = Calendar::default();

        calendar.update(Message::NextMonth);
        calendar.update(Message::NextMonth);
        calendar.update(Message::Today);

        assert_eq!(calendar.state(), &CalendarState::current());
    }

    #[test]
    fn the_week_starts_on_monday() {
        assert_eq!(WEEKDAYS.len(), 7);
        assert_eq!(WEEKDAYS[0], "Mon");
        assert_eq!(WEEKDAYS[6], "Sun");
    }

    #[test]
    fn the_box_is_wider_than_the_grid_it_wraps() {
        assert!(Calendar::content_width(10.0) > 0.0);
        assert!(Calendar::content_height() > 0.0);
    }
}
