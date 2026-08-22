//! The drawing of one panel: a heading, a rule and the lines under it.

use iced::{
    Alignment, Color, Element, Length,
    widget::{Column, Row, Space, container, text}
};

use super::readings::Panel;
use crate::app::Message;

/// Which edge of the canvas a column is pinned to.
///
/// The columns face outwards, the way the wallpaper's own margins run: the
/// left one reads label first, the right one value first, so both leave the
/// middle of the screen to the hour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    /// Pinned to the left edge, label before value.
    Leading,
    /// Pinned to the right edge, value before label.
    Trailing,
    /// Standing in the middle, where the centre of the strip was.
    Middle
}

impl Side {
    /// The padding that holds a block `inwards` from its own edge.
    ///
    /// The lane a block stands in: the edge sections push their near blocks
    /// away from the edge they belong to, and the middle one stands where it
    /// is.
    pub(super) fn lane(self, inwards: f32) -> iced::Padding {
        match self {
            Self::Leading => iced::Padding::default().left(inwards),
            Self::Trailing => iced::Padding::default().right(inwards),
            Self::Middle => iced::Padding::default()
        }
    }

    /// How a column of this side lines its content up.
    pub(super) const fn alignment_x(self) -> Alignment {
        match self {
            Self::Leading => Alignment::Start,
            Self::Trailing => Alignment::End,
            Self::Middle => Alignment::Center
        }
    }
}

/// Ink of a panel: what its headings, labels and values are drawn in.
#[derive(Debug, Clone, Copy)]
pub(super) struct Ink {
    /// Colour the values are drawn in, the strongest on the canvas.
    pub(super) value: Color,
    /// Size the body lines are drawn at.
    pub(super) size:  f32
}

impl Ink {
    /// Colour of the headings, a step back from the values.
    fn heading(self) -> Color {
        self.value.scale_alpha(0.85)
    }

    /// Colour of the labels, the quietest ink of the canvas.
    fn label(self) -> Color {
        self.value.scale_alpha(0.55)
    }
}

/// Draws one panel: heading, rule, then a line per reading.
///
/// `bloom` is how far the block has written itself out. It writes out from
/// the top, the readings appearing as the room for them grows, rather than a
/// line at a time: a line is a step, and a dozen steps in a fifth of a second
/// is what the eye reads as juddering.
pub(super) fn panel<'a>(panel: &Panel, side: Side, ink: Ink, bloom: f32) -> Element<'a, Message> {
    revealed(
        written(panel, side, ink),
        room(panel.rows.len(), ink.size * 1.4, ink),
        bloom
    )
}

/// The lines of a panel, in the ink they are drawn in.
fn written<'a>(panel: &Panel, side: Side, ink: Ink) -> Element<'a, Message> {
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
            .chain(lines)
    )
    .spacing(ink.size * 0.28)
    .width(Length::Fill)
    .align_x(side.alignment_x())
    .into()
}

/// The room a block of `rows` lines, each `line` tall, takes when it is open.
///
/// A heading, the rule under it and a line per reading, with the column's own
/// gap between each of them. Stated rather than measured because the room has
/// to be taken before there is anything in it: see [`revealed`].
#[expect(
    clippy::cast_precision_loss,
    reason = "a block holds a handful of rows, far below any precision limit"
)]
fn room(rows: usize, line: f32, ink: Ink) -> f32 {
    let heading = ink.size * 1.05 * 1.4;
    let gaps = (rows + 1) as f32 * (ink.size * 0.28);

    (rows as f32).mul_add(line, heading + 1.0 + gaps)
}

/// Opens `shown` from the top inside the room it will need when it is open.
///
/// A block that grew as it opened pushed everything below it down the column,
/// one layout per frame, all the way through the opening — which is the
/// juddering the whole canvas had, and it landed on the lower blocks worst
/// because every block above them was growing at once. The room is taken in
/// full from the first frame instead, and the opening changes only how much
/// of it has been written into. Nothing on the canvas moves while a block
/// opens.
fn revealed(shown: Element<'_, Message>, full: f32, bloom: f32) -> Element<'_, Message> {
    container(
        container(shown)
            .max_height(full * bloom.clamp(0.0, 1.0))
            .clip(true)
    )
    .height(Length::Fixed(full))
    .clip(true)
    .into()
}

/// The block of a module that has nothing to say yet.
///
/// Every module opens into the same shape — a heading, a rule and lines
/// under it — and one with no readings of its own would otherwise open into
/// nothing and break the shape of the column it stands in. It opens into the
/// shape with the lines left blank: two dim bars where the readings would
/// stand, which is a placeholder anywhere anyone has seen one.
pub(super) fn awaited<'a>(title: &str, side: Side, ink: Ink, bloom: f32) -> Element<'a, Message> {
    revealed(
        blank(title, side, ink),
        room(2, ink.size * 0.55, ink),
        bloom
    )
}

/// The blank shape a module with nothing to say opens into.
fn blank<'a>(title: &str, side: Side, ink: Ink) -> Element<'a, Message> {
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

/// The month grid, opening the way every other block does.
///
/// The grid is six rows against the one row of the island above it, so what
/// it needs is stated rather than measured: the room is taken from the first
/// frame and the grid is written into it, the same as a panel of readings.
pub(super) fn month(grid: Element<'_, Message>, ink: Ink, bloom: f32) -> Element<'_, Message> {
    revealed(grid, ink.size * MONTH_ROWS, bloom)
}

/// How many lines of the body ink the month grid stands.
///
/// A heading, a row of weekday names and six weeks, each a line and a little,
/// with the grid's own padding around them — measured off the grid itself at
/// the body size and left a little over, because a figure short of the truth
/// does not merely open early, it clips the last week off for good.
const MONTH_ROWS: f32 = 17.0;

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
fn rule<'a>(ink: Ink) -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(ink.label().into()),
            ..container::Style::default()
        })
        .into()
}

/// One reading: its label and its value, pushed to opposite edges.
fn line<'a>(label: &str, value: &str, side: Side, ink: Ink) -> Element<'a, Message> {
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
