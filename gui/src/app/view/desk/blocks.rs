//! The drawing of one panel: a heading, a rule and the lines under it.
//!
//! Seven rooms. Here are the blocks themselves — the three shapes a unit opens
//! into; [`room`] is how much of the column a block takes and how it is
//! written into it, [`parts`] is the pieces every shape is built from, and
//! [`accordion`], [`choices`], [`overview`] and [`trace`] draw the readings
//! that are shapes rather than tables.

mod accordion;
mod choices;
mod overview;
mod parts;
mod room;
mod trace;

use iced::{Alignment, Color, Element};

pub(in crate::app::view::desk) use self::room::MONTH_ROWS;
use self::{
    parts::{blank, written},
    room::{revealed, room}
};
use super::readings::{Figure, Panel};
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
/// The room one panel takes when it is open.
pub(in crate::app::view::desk) fn room_of(panel: &Panel, ink: Ink) -> f32 {
    room(panel.rows.len(), ink.size * 1.4, ink) + drawing_room(panel, ink)
}

/// The room the blank shape takes, which is a heading and two bars.
pub(in crate::app::view::desk) fn blank_room(ink: Ink) -> f32 {
    room(2, ink.size * 0.55, ink)
}

/// The room the drawing of a panel takes, nothing when it carries none.
fn drawing_room(panel: &Panel, ink: Ink) -> f32 {
    match panel.figure.as_ref() {
        Some(Figure::Picture(_)) => ink.size.mul_add(0.28, room::picture(ink)),
        Some(Figure::Overview {
            rooms, ..
        }) => ink.size.mul_add(0.28, overview::room(ink, rooms.len())),
        Some(Figure::Accordion(_)) => ink.size.mul_add(0.28, accordion::room(ink)),
        Some(Figure::Choices(offered)) => {
            ink.size.mul_add(0.28, choices::room(offered.len(), ink))
        }
        Some(Figure::Trace {
            ..
        }) => ink.size.mul_add(0.28, trace::room(ink)),
        None => 0.0
    }
}

pub(super) fn panel<'a>(panel: &Panel, side: Side, ink: Ink, bloom: f32) -> Element<'a, Message> {
    revealed(written(panel, side, ink), room_of(panel, ink), bloom)
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

/// The month grid, opening the way every other block does.
///
/// The grid is six rows against the one row of the island above it, so what
/// it needs is stated rather than measured: the room is taken from the first
/// frame and the grid is written into it, the same as a panel of readings.
pub(super) fn month(grid: Element<'_, Message>, ink: Ink, bloom: f32) -> Element<'_, Message> {
    revealed(grid, ink.size * MONTH_ROWS, bloom)
}
