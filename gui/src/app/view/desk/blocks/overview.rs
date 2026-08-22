//! The workspaces of a screen, drawn as the rooms they are.
//!
//! A list saying "three windows" is a fact about a workspace; a miniature of
//! it is the workspace itself, and the eye picks the one it wants out of a row
//! of them without reading a word. Every screen that ever offered an overview
//! drew it this way, and the compositor already knows where every window
//! stands, so nothing is guessed here — the shapes are the layout, to scale.
//!
//! The row is the same shape the pictures of the desktop take: the workspace
//! in view stands biggest in the middle of it and its neighbours fall away to
//! either side, so which one the user is standing in is read from the shape of
//! the row rather than from a highlight on one box of several equal boxes.
//!
//! Two rooms: here is the row and the sizes it gives each miniature, and
//! [`screen`] draws one workspace with the windows standing on it.

mod screen;

use hydebar_core::modules::desk::looks::centred;
use iced::{
    Alignment, Element, Length,
    widget::{Column, Row, container, text}
};

use super::{super::readings::Miniature, Ink};
use crate::app::Message;

/// How many workspaces stand on either side of the one in view.
///
/// Two: a miniature is a screen laid on its side, so it is half again as wide
/// as it is tall, and a row of seven of them leaves each one narrower than the
/// windows drawn in it.
const REACH: usize = 2;

/// Aspect of the miniature, wider than tall the way a screen is.
const ASPECT: f32 = 16.0 / 9.0;

/// How wide the row may be, in body letters.
///
/// The measure a block is written to, less the margin a row of pictures wants
/// inside it: the row is sized to fit rather than clipped to fit, because a
/// clipped miniature is a workspace with windows cut off it.
const BUDGET: f32 = 26.0;

/// The tallest the workspace in view is drawn, as a share of the body ink.
const TALLEST: f32 = 7.0;

/// The shortest it is drawn, however many workspaces share the row.
const SHORTEST: f32 = 2.0;

/// How tall a workspace stands at each remove from the one in view.
const SMALLER: [f32; REACH] = [0.55, 0.38];

/// Gap kept between two miniatures, in body letters.
const GAP: f32 = 0.4;

/// How tall the caption under a miniature stands, as a share of the body ink.
const CAPTION: f32 = 1.4;

/// The room a row of `rooms` workspaces takes, at the given ink.
pub(super) fn room(ink: Ink, rooms: usize) -> f32 {
    ink.size
        .mul_add(0.28, ink.size * (tallest(shown(rooms)) + CAPTION))
}

/// Draws the workspaces, the one in view biggest in the middle of the row.
pub(super) fn overview<'a>(
    rooms: &[Miniature],
    ground: Option<&iced::widget::image::Handle>,
    ink: Ink
) -> Element<'a, Message> {
    let at = rooms
        .iter()
        .position(|workspace| workspace.active)
        .unwrap_or_default();
    let drawn = centred(rooms.len(), at, REACH);
    let middle = drawn.len() / 2;
    let tall = ink.size * tallest(drawn.len());

    let row = Row::with_children(drawn.iter().enumerate().filter_map(|(place, &index)| {
        let workspace = rooms.get(index)?;

        Some(slide(
            workspace,
            ground,
            tall * smaller(place.abs_diff(middle)),
            ink
        ))
    }))
    .spacing(ink.size * GAP)
    .align_y(Alignment::End);

    container(row)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

/// How many of `rooms` workspaces the row draws.
fn shown(rooms: usize) -> usize {
    (REACH * 2 + 1).min(rooms)
}

/// How tall a workspace stands, `away` places from the one in view.
fn smaller(away: usize) -> f32 {
    if away == 0 {
        return 1.0;
    }

    SMALLER.get(away - 1).copied().unwrap_or(SMALLER[REACH - 1])
}

/// How tall the workspace in view stands when `shown` of them share the row.
///
/// The row is given a measure and the miniatures are sized to fill it, so two
/// workspaces are drawn large and five are drawn small rather than five being
/// drawn off the side of the block. It never grows past [`TALLEST`]: a single
/// workspace blown up to the width of the column is a picture of a wallpaper,
/// not an overview.
fn tallest(shown: usize) -> f32 {
    if shown == 0 {
        return 0.0;
    }

    let middle = shown / 2;
    let spread: f32 = (0..shown)
        .map(|place| smaller(place.abs_diff(middle)))
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a screen holds a handful of workspaces"
    )]
    let gaps = GAP * (shown - 1) as f32;

    ((BUDGET - gaps) / (spread * ASPECT)).clamp(SHORTEST, TALLEST)
}

/// One workspace of the row: the room itself and its name under it.
fn slide<'a>(
    workspace: &Miniature,
    ground: Option<&iced::widget::image::Handle>,
    tall: f32,
    ink: Ink
) -> Element<'a, Message> {
    let wide = tall * ASPECT;
    let name = text(workspace.name.clone())
        .size(ink.size * 0.9)
        .color(if workspace.active {
            ink.value
        } else {
            ink.label()
        });

    Column::new()
        .push(screen::drawn(workspace, ground, wide, tall, ink))
        .push(name)
        .spacing(ink.size * 0.28)
        .align_x(Alignment::Center)
        .into()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    fn ink() -> Ink {
        Ink {
            value: iced::Color::WHITE,
            size:  14.0
        }
    }

    /// The whole point of the shape: the eye is meant to land on one room.
    #[test]
    fn the_workspace_in_view_is_the_tallest_of_the_row() {
        assert!(smaller(0) > smaller(1));
        assert!(smaller(1) > smaller(2));
        assert_eq!(smaller(REACH + 4), SMALLER[REACH - 1]);
    }

    /// A row wider than the block is a workspace with its windows cut off.
    #[test]
    fn the_row_is_sized_to_the_measure_it_is_written_to() {
        for rooms in 1..12_usize {
            let shown = shown(rooms);
            let middle = shown / 2;
            let tall = tallest(shown);
            #[expect(clippy::cast_precision_loss, reason = "a handful of rooms")]
            let width: f32 = GAP.mul_add(
                (shown - 1) as f32,
                (0..shown)
                    .map(|place| tall * smaller(place.abs_diff(middle)) * ASPECT)
                    .sum::<f32>()
            );

            assert!(
                width <= BUDGET + 0.01,
                "{rooms} rooms are drawn {width} letters wide"
            );
        }
    }

    #[test]
    fn a_screen_of_one_workspace_is_not_blown_up_to_a_wallpaper() {
        assert_eq!(tallest(1), TALLEST);
    }

    #[test]
    fn the_room_holds_the_miniature_and_the_name_under_it() {
        let asked = room(ink(), 3);

        assert!(asked > 14.0 * tallest(3), "the name stands under the row");
        assert!(asked < 14.0 * (tallest(3) + CAPTION + 1.0));
    }

    #[test]
    fn a_screen_with_no_workspaces_asks_for_no_room() {
        assert_eq!(tallest(0), 0.0);
    }
}
