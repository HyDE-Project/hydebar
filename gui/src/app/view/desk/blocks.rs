//! The drawing of one panel: a heading, a rule and the lines under it.
//!
//! Three rooms. Here are the blocks themselves — the three shapes a unit
//! opens into; [`room`] is how much of the column a block takes and how it is
//! written into it, and [`parts`] is the pieces every shape is built from.

mod parts;
mod room;

use iced::{Alignment, Color, Element};

use self::{
    parts::{blank, written},
    room::{MONTH_ROWS, revealed, room}
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
