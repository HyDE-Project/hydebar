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
