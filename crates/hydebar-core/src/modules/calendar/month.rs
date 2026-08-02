//! What a month is: days, shape and names. No rendering in this room.

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
    #[must_use]
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

    #[must_use]
    pub const fn year(&self) -> i32 {
        self.year
    }

    /// Month in view, 1–12.
    #[must_use]
    pub const fn month(&self) -> u32 {
        self.month
    }

    pub const fn previous_month(&mut self) {
        if self.month == 1 {
            self.month = 12;
            self.year -= 1;
        } else {
            self.month -= 1;
        }
    }

    pub const fn next_month(&mut self) {
        if self.month == 12 {
            self.month = 1;
            self.year += 1;
        } else {
            self.month += 1;
        }
    }

    /// English name of the month in view.
    #[must_use]
    pub fn month_name(&self) -> &'static str {
        u8::try_from(self.month)
            .ok()
            .and_then(|month| Month::try_from(month).ok())
            .map_or("Unknown", |month| month.name())
    }

    /// The grid of days this month shows.
    #[must_use]
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
    ///
    /// # Panics
    ///
    /// Panics when `year` lies outside the range `chrono::NaiveDate`
    /// supports, so even the fallback first of January cannot be built.
    #[must_use]
    pub fn generate(year: i32, month: u32) -> Self {
        let today = Local::now().date_naive();

        let first_day = NaiveDate::from_ymd_opt(year, month, 1)
            .or_else(|| NaiveDate::from_ymd_opt(year, 1, 1))
            .unwrap_or_default();
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

        let remaining = u32::try_from(42 - days.len()).unwrap_or(0);

        for day in 1..=remaining {
            days.push(DayInfo {
                day,
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
                .and_then(|next| u32::try_from(next.signed_duration_since(date).num_days()).ok())
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
            Self::InvalidMonth {
                month
            } => {
                write!(f, "invalid month: {month}, must be in range 1-12")
            }
        }
    }
}

impl std::error::Error for CalendarError {}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Local};

    use super::*;

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
}
