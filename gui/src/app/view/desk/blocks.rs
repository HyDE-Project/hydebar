//! The drawing of one panel: a heading, a rule and the lines under it.
//!
//! Seven rooms. Here are the blocks themselves — the three shapes a unit opens
//! into; [`room`] is how much of the column a block takes and how it is
//! written into it, [`parts`] is the pieces every shape is built from, and
//! [`accordion`], [`choices`], [`overview`] and [`trace`] draw the readings
//! that are shapes rather than tables. [`ink`] carries the vocabulary all of
//! them draw with: the side a column sits on, how far a block has travelled,
//! and the ink of its lines.

mod accordion;
mod choices;
mod ink;
mod overview;
mod parts;
mod room;
mod trace;

use iced::Element;

pub(in crate::app::view::desk) use self::{
    ink::{Along, Ink, Side},
    room::MONTH_ROWS
};
use self::{
    parts::{blank, written},
    room::{revealed, room}
};
use super::readings::{Figure, Panel};
use crate::app::Message;

/// Draws one panel: heading, rule, then a line per reading.
///
/// `bloom` is how far the block has written itself out. It writes out from
/// the top, the readings appearing as the room for them grows, rather than a
/// line at a time: a line is a step, and a dozen steps in a fifth of a second
/// is what the eye reads as juddering.
/// The room one panel takes when it is open.
///
/// `headed` is whether it writes its own name: the first block of a unit
/// gives that name to the row the island stands in, and asks for a heading
/// less room here.
pub(in crate::app::view::desk) fn room_of(panel: &Panel, ink: Ink, headed: bool) -> f32 {
    let written = room(panel.rows.len(), ink.size * 1.4, ink) + drawing_room(panel, ink);

    if headed {
        written
    } else {
        written - self::room::heading(ink)
    }
}

/// The room the blank shape takes, which is two bars under a rule.
pub(in crate::app::view::desk) fn blank_room(ink: Ink) -> f32 {
    room(2, ink.size * 0.55, ink) - self::room::heading(ink)
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

pub(super) fn panel<'a>(
    panel: &Panel,
    side: Side,
    ink: Ink,
    bloom: f32,
    headed: bool
) -> Element<'a, Message> {
    revealed(
        written(panel, side, ink, headed),
        room_of(panel, ink, headed),
        bloom
    )
}

/// The block of a module that has nothing to say yet.
///
/// Every module opens into the same shape — a rule and lines under it, its
/// name up on the island's own row — and one with no readings of its own
/// would otherwise open into nothing and break the shape of the column it
/// stands in. It opens into the shape with the lines left blank: two dim bars
/// where the readings would stand, which is a placeholder anywhere anyone has
/// seen one.
pub(super) fn awaited<'a>(side: Side, ink: Ink, bloom: f32) -> Element<'a, Message> {
    revealed(blank(side, ink), blank_room(ink), bloom)
}

/// The name of a block, written in beside the island it arrived as.
///
/// Opened rather than simply drawn, the way the lines under it are: it is
/// part of the block, not part of the island, and a name standing there while
/// its block was still on its way would be the one piece of the canvas that
/// arrived early.
pub(in crate::app::view::desk) fn name<'a>(
    heading: &str,
    ink: Ink,
    bloom: f32
) -> Element<'a, Message> {
    revealed(
        iced::widget::text(heading.to_uppercase())
            .size(ink.size * 1.05)
            .color(ink.heading())
            .into(),
        room::heading(ink),
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
