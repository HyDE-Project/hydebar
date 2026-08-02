//! Drawing of the month: header, weekday row, day grid and their styles.

use iced::{
    Alignment, Border, Color, Element, Length, Theme,
    widget::{Column, Row, button, column, container, row, rule}
};

use super::{
    Message,
    metrics::{
        CELL, CELL_GAP, DAY_SIZE, OUTER_PADDING, SECTION_GAP, TITLE_SIZE, WEEKDAY_SIZE, WEEKDAYS,
        grid_width
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

    let weekday_header = Row::with_children(WEEKDAYS.iter().map(|day| {
        container(text(*day).size(scale::scaled(WEEKDAY_SIZE)))
            .width(Length::Fixed(scale::scaled(CELL)))
            .align_x(Alignment::Center)
            .into()
    }))
    .spacing(scale::scaled(CELL_GAP));

    let calendar_data = state.generate_calendar();
    let week_rows = calendar_data
        .days
        .chunks(7)
        .map(|week| {
            Row::with_children(week.iter().map(day_cell))
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::widget::button::Status;
    use iced_test::simulator;

    use super::*;

    fn view() -> Element<'static, Message> {
        let state = CalendarState::new(2026, 8).expect("August 2026 is a month");

        month_view_owned(state)
    }

    /// The month view over a state the test owns for as long as the view
    /// lives, which is what the simulator needs.
    fn month_view_owned(state: CalendarState) -> Element<'static, Message> {
        let state = Box::leak(Box::new(state));
        let icons = Box::leak(Box::new(IconTheme::default()));

        month_view(state, icons)
    }

    #[test]
    fn the_month_view_names_the_month_and_the_year() {
        let mut ui = simulator(view());

        assert!(ui.find("August 2026").is_ok());
        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn every_weekday_heads_its_column() {
        let mut ui = simulator(view());

        for day in WEEKDAYS {
            assert!(ui.find(day.to_owned()).is_ok(), "the {day} column is headed");
        }
    }

    #[test]
    fn pressing_the_title_returns_to_today() {
        let mut ui = simulator(view());
        let _ = ui.click("August 2026").expect("the title is a button");

        let published: Vec<Message> = ui.into_messages().collect();
        assert_eq!(published, vec![Message::Today]);
    }

    #[test]
    fn a_day_of_the_grid_is_drawn_for_every_cell_of_six_weeks() {
        let state = CalendarState::new(2026, 8).expect("August 2026 is a month");
        let days = state.generate_calendar().days;

        assert_eq!(days.len() % 7, 0);
        assert!(days.iter().any(|day| day.in_month));
        assert!(days.iter().any(|day| !day.in_month));
    }

    #[test]
    fn the_grid_is_drawn_for_a_month_that_holds_today() {
        let mut ui = simulator(month_view_owned(CalendarState::current()));

        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn a_resting_chevron_paints_nothing() {
        let resting = nav_button_style(&Theme::Dark, Status::Active);

        assert!(resting.background.is_none());
        assert_eq!(resting.border.width, 0.0);
        assert_eq!(resting.border.color, Color::TRANSPARENT);
        assert_eq!(resting.text_color, Theme::Dark.palette().text);
    }

    #[test]
    fn a_hovered_chevron_fills_with_the_weak_background() {
        let theme = Theme::Dark;
        let hovered = nav_button_style(&theme, Status::Hovered);

        assert_eq!(
            hovered.background,
            Some(theme.extended_palette().background.weak.color.into())
        );
    }

    #[test]
    fn pressed_and_disabled_chevrons_rest_like_an_idle_one() {
        for status in [Status::Pressed, Status::Disabled] {
            assert!(nav_button_style(&Theme::Dark, status).background.is_none());
        }
    }

    #[test]
    fn today_is_the_one_filled_cell() {
        let theme = Theme::Dark;
        let today = day_button_style(&theme, Status::Active, true, true);

        assert_eq!(today.background, Some(theme.palette().primary.into()));
        assert_eq!(today.text_color, theme.extended_palette().primary.base.text);
    }

    #[test]
    fn a_day_of_this_month_sits_flat_in_the_bar_text() {
        let theme = Theme::Dark;
        let day = day_button_style(&theme, Status::Active, true, false);

        assert!(day.background.is_none());
        assert_eq!(day.text_color, theme.palette().text);
    }

    #[test]
    fn a_day_of_a_neighbouring_month_is_dimmed() {
        let theme = Theme::Dark;
        let outside = day_button_style(&theme, Status::Active, false, false);

        assert!(outside.background.is_none());
        assert_eq!(
            outside.text_color,
            theme.extended_palette().background.weak.text
        );
    }

    #[test]
    fn hovering_a_day_lights_it_up() {
        let theme = Theme::Dark;
        let hovered = day_button_style(&theme, Status::Hovered, true, false);
        let palette = theme.extended_palette();

        assert_eq!(hovered.background, Some(palette.primary.weak.color.into()));
        assert_eq!(hovered.text_color, palette.primary.weak.text);
    }

    #[test]
    fn hovering_today_leaves_it_filled_as_it_was() {
        let theme = Theme::Dark;
        let hovered = day_button_style(&theme, Status::Hovered, true, true);

        assert_eq!(hovered.background, Some(theme.palette().primary.into()));
    }
}
