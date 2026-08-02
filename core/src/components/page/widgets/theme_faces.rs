//! The two faces a theme card wears: a line in a list, or a tile in a
//! grid. Both read their colours through the same paint so a card looks
//! the same whichever way the page lays it.

use std::path::PathBuf;

use iced::{
    Alignment, Element, Length, Theme,
    widget::{Column, Row, button, container, text}
};

use super::theme_card::{
    ChipPaint, DOT_GAP_EM, ThemeChip, busy_strip, card_colors, palette_dots
};
use crate::components::icons::icon_raw_sized;

/// The screenshot of a theme, sized to the room its face gives it.
fn preview(path: &PathBuf, height: f32) -> iced::widget::Image<iced::widget::image::Handle> {
    iced::widget::image(iced::widget::image::Handle::from_path(path))
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .content_fit(iced::ContentFit::Cover)
}

/// The face of a card drawn as one line of a list: name, thumbnail and
/// palette side by side, deeds at the end, the busy strip underneath.
#[expect(
    clippy::too_many_arguments,
    reason = "the face states everything one card line shows in one call"
)]
pub(super) fn horizontal_face<'a, M: Clone + 'static>(
    label: String,
    badge: Option<&'static str>,
    screenshot: Option<&PathBuf>,
    paint: Option<&ChipPaint>,
    paint_colors: Option<(iced::Color, iced::Color, iced::Color)>,
    state: ThemeChip,
    control: f32,
    deeds: Option<Row<'a, M>>
) -> Element<'a, M> {
    let mut name_row = Row::new()
        .spacing(DOT_GAP_EM * control * 0.5)
        .align_y(Alignment::Center)
        .push(text(label).size(control));

    if let Some(glyph) = badge {
        name_row = name_row.push(icon_raw_sized(glyph.to_owned(), Some(control * 0.8)));
    }

    let mut face = Row::new()
        .spacing(DOT_GAP_EM * control)
        .align_y(Alignment::Center)
        .push(container(name_row).width(Length::Fill))
        .width(Length::Fill);

    if let Some(thumb) = screenshot.map(|path| preview(path, control * 2.2)) {
        face = face.push(container(thumb).width(Length::Fixed(control * 4.0)));
    }

    if let Some(paint) = paint {
        face = face.push(
            container(palette_dots::<M>(paint.palette.clone(), control)).width(Length::Fill)
        );
    }

    let pressable = button(face)
        .padding(0)
        .style(move |theme: &Theme, _| button::Style {
            background: None,
            text_color: card_colors(theme, paint_colors, state).1,
            ..button::Style::default()
        })
        .width(Length::Fill);

    let mut line = Row::new()
        .align_y(Alignment::Center)
        .spacing(DOT_GAP_EM * control)
        .push(pressable);

    if let Some(deeds) = deeds {
        line = line.push(deeds);
    }

    let mut column = Column::new().push(line).spacing(DOT_GAP_EM * control);

    if let ThemeChip::Applying(spinner) = state {
        column = column.push(busy_strip(spinner, control));
    }

    column.into()
}

/// The face of a card drawn as a tile of the grid: screenshot, name and
/// palette stacked, deeds under the tile, the busy strip in the stack.
#[expect(
    clippy::too_many_arguments,
    reason = "the face states everything one card tile shows in one call"
)]
pub(super) fn vertical_face<'a, M: Clone + 'static>(
    label: String,
    badge: Option<&'static str>,
    screenshot: Option<&PathBuf>,
    paint: Option<&ChipPaint>,
    paint_colors: Option<(iced::Color, iced::Color, iced::Color)>,
    state: ThemeChip,
    control: f32,
    cell: f32,
    deeds: Option<Row<'a, M>>
) -> Element<'a, M> {
    let name: Element<'a, M> = if let Some(glyph) = badge {
        container(
            Row::new()
                .spacing(DOT_GAP_EM * control * 0.5)
                .align_y(Alignment::Center)
                .push(text(label).size(control))
                .push(icon_raw_sized(glyph.to_owned(), Some(control * 0.8)))
        )
        .width(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
    } else {
        text(label)
            .size(control)
            .width(iced::Length::Fill)
            .align_x(iced::Alignment::Center)
            .into()
    };

    let body: Element<'a, M> = match paint {
        Some(paint) => {
            let mut column = Column::new().spacing(DOT_GAP_EM * control);

            if let Some(shot) = screenshot.map(|path| preview(path, cell * 0.5)) {
                column = column.push(shot);
            }

            let mut column = column
                .push(name)
                .push(palette_dots(paint.palette.clone(), control));

            if let ThemeChip::Applying(spinner) = state {
                column = column.push(busy_strip(spinner, control));
            }

            column.into()
        }
        None => name
    };

    let pressable = button(container(body).width(Length::Fill))
        .padding(0)
        .style(move |theme: &Theme, _| button::Style {
            background: None,
            text_color: card_colors(theme, paint_colors, state).1,
            ..button::Style::default()
        })
        .width(Length::Fill);

    let mut column = Column::new()
        .push(pressable)
        .spacing(DOT_GAP_EM * control)
        .align_x(Alignment::Center);

    if let Some(deeds) = deeds {
        column = column.push(deeds);
    }

    column.into()
}
