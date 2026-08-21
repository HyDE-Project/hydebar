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
/// `bloom` is how far the block has written itself out, and it writes out one
/// line at a time rather than all at once behind a fade: the heading and its
/// rule come first, then a reading, then the next, the way a monitor filling
/// in reads.
pub(super) fn panel<'a>(panel: &Panel, side: Side, ink: Ink, bloom: f32) -> Element<'a, Message> {
    let heading = text(panel.title.to_uppercase())
        .size(ink.size * 1.05)
        .color(ink.heading());

    let rule = container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(ink.label().into()),
            ..container::Style::default()
        });

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "a block holds a handful of rows, far below any precision limit"
    )]
    let written = (bloom.clamp(0.0, 1.0) * panel.rows.len() as f32).ceil() as usize;

    let lines = panel
        .rows
        .iter()
        .take(written)
        .map(|(label, value)| line(label, value, side, ink));

    Column::with_children(
        std::iter::once(heading.into())
            .chain(std::iter::once(rule.into()))
            .chain(lines)
    )
    .spacing(ink.size * 0.28)
    .width(Length::Fill)
    .align_x(side.alignment_x())
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
