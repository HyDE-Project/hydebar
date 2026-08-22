//! The pieces every shape of block is built from.

use iced::{
    Alignment, Element, Length,
    widget::{Column, Row, Space, container, text}
};

use super::{
    super::{
        super::super::state::Message,
        readings::{Figure, Panel}
    },
    Ink, Side,
    accordion::accordion,
    overview::overview,
    trace::trace
};

/// The lines of a panel, in the ink they are drawn in.
pub(super) fn written<'a>(panel: &Panel, side: Side, ink: Ink) -> Element<'a, Message> {
    let heading = text(panel.title.to_uppercase())
        .size(ink.size * 1.05)
        .color(ink.heading());

    let lines = panel
        .rows
        .iter()
        .map(|(label, value)| line(label, value, side, ink));

    Column::with_children(
        std::iter::once(heading.into())
            .chain(std::iter::once(rule(ink)))
            .chain(panel.figure.as_ref().map(|figure| drawn(figure, ink)))
            .chain(lines)
    )
    .spacing(ink.size * 0.28)
    .width(Length::Fill)
    .align_x(side.alignment_x())
    .into()
}

/// The drawing a panel carries, in the room its kind asks for.
fn drawn<'a>(figure: &Figure, ink: Ink) -> Element<'a, Message> {
    match figure {
        Figure::Picture(handle) => container(
            iced::widget::image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fixed(super::room::picture(ink)))
                .content_fit(iced::ContentFit::Cover)
        )
        .clip(true)
        .into(),
        Figure::Overview(workspaces) => overview(workspaces, ink),
        Figure::Accordion(reel) => accordion(reel, ink),
        Figure::Trace {
            readings,
            ceiling
        } => trace(readings, *ceiling, ink)
    }
}

/// The blank shape a module with nothing to say opens into.
pub(super) fn blank<'a>(title: &str, side: Side, ink: Ink) -> Element<'a, Message> {
    let heading = text(title.to_uppercase())
        .size(ink.size * 1.05)
        .color(ink.heading());

    let lines = [0.62_f32, 0.38].into_iter().map(|share| {
        Row::with_children(match side {
            Side::Leading | Side::Middle => vec![
                bar(ink.size * 4.0, ink),
                Space::new().width(Length::Fill).into(),
                bar(ink.size * 9.0 * share, ink),
            ],
            Side::Trailing => vec![
                bar(ink.size * 9.0 * share, ink),
                Space::new().width(Length::Fill).into(),
                bar(ink.size * 4.0, ink),
            ]
        })
        .width(Length::Fill)
        .spacing(ink.size)
        .align_y(Alignment::Center)
        .into()
    });

    Column::with_children(
        std::iter::once(heading.into())
            .chain(std::iter::once(rule(ink)))
            .chain(lines)
    )
    .spacing(ink.size * 0.28)
    .width(Length::Fill)
    .align_x(side.alignment_x())
    .into()
}

/// One blank where a reading will stand, `width` wide.
fn bar<'a>(width: f32, ink: Ink) -> Element<'a, Message> {
    container(
        Space::new()
            .width(Length::Fixed(width.max(ink.size)))
            .height(Length::Fixed(ink.size * 0.55))
    )
    .style(move |_| container::Style {
        background: Some(ink.value.scale_alpha(0.18).into()),
        border: iced::Border::default().rounded(ink.size * 0.2),
        ..container::Style::default()
    })
    .into()
}

/// The thin line a heading is underscored with.
pub(super) fn rule<'a>(ink: Ink) -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(ink.label().into()),
            ..container::Style::default()
        })
        .into()
}

/// One reading: its label and its value, pushed to opposite edges.
///
/// A line with no label is not a reading but a continuation — a line of a
/// tooltip, the name of a tray item — and it stands on the column's own edge
/// instead of being flung across the block to where a value would sit.
fn line<'a>(label: &str, value: &str, side: Side, ink: Ink) -> Element<'a, Message> {
    if label.is_empty() {
        return container(text(value.to_owned()).size(ink.size).color(ink.value))
            .width(Length::Fill)
            .align_x(side.alignment_x())
            .into();
    }

    let label = text(label.to_owned()).size(ink.size).color(ink.label());
    let value = text(value.to_owned()).size(ink.size).color(ink.value);

    let children: Vec<Element<'a, Message>> = match side {
        Side::Leading | Side::Middle => vec![
            label.into(),
            Space::new().width(Length::Fill).into(),
            value.into(),
        ],
        Side::Trailing => vec![
            value.into(),
            Space::new().width(Length::Fill).into(),
            label.into(),
        ]
    };

    Row::with_children(children)
        .width(Length::Fill)
        .spacing(ink.size)
        .align_y(Alignment::Center)
        .into()
}
