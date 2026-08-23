//! The choices a button module opens into, written out the canvas way.
//!
//! Some modules are not readings at all. A power button knows nothing and
//! reports nothing; what it has is a menu, and what the canvas can show of it
//! is the choices that menu offers.
//!
//! Not the row of pills the strip would draw. The strip has a bar's worth of
//! room and answers a glance, so it folds a whole menu into one glyph; the
//! canvas has a column and answers a look from across the room, so it opens
//! the menu out — every choice at a size the eye finds without hunting, each
//! under the name the desktop calls it by. Drawing the strip's own pills here
//! would give the two shapes of the bar one shape between them, which is the
//! whole of what the canvas is for.
//!
//! The choices share the block's width in even places rather than standing at
//! their own widths: a block is as wide as the column allows it, and a row
//! that took the width its own writing needed ran out past the column and
//! over whatever stood beside it.

use iced::{
    Alignment, Element, Length,
    widget::{Column, Row, Space, text}
};

use super::{super::super::super::state::Message, Ink, Side};

/// How big a choice's glyph is drawn, as a share of the body ink.
///
/// Twice the writing beside it. A menu is what the module is for, and a glyph
/// at the size of a reading reads as one more reading.
const GLYPH: f32 = 2.1;

/// How big the name under a glyph is written, as a share of the body ink.
const NAME: f32 = 0.72;

/// The gap between a glyph and its name, as a share of the body ink.
const UNDER: f32 = 0.25;

/// How many choices stand on one line before the next one starts.
///
/// Six across a column this wide leaves each of them room for its name; more
/// than that and the names run into one another, which is the one thing a
/// caption cannot do.
const ACROSS: usize = 6;

/// How many lines a name is given, before its place is settled.
const LINES: f32 = 2.0;

/// The room one line of choices takes, at the given ink.
fn line(ink: Ink) -> f32 {
    ink.size * NAME.mul_add(LINES, GLYPH + UNDER) * 1.1
}

/// The room `count` choices take, at the given ink.
pub(super) fn room(count: usize, ink: Ink) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "a menu holds a few choices")]
    let lines = count.div_ceil(ACROSS).max(1) as f32;

    lines.mul_add(line(ink), (lines - 1.0) * ink.size * 0.4)
}

/// Draws the choices in even places, line by line.
pub(super) fn choices<'a>(
    choices: &[(String, String)],
    side: Side,
    ink: Ink
) -> Element<'a, Message> {
    Column::with_children(choices.chunks(ACROSS).map(|line| across(line, side, ink)))
        .spacing(ink.size * 0.4)
        .width(Length::Fill)
        .into()
}

/// One line of choices, every place the same width as the next.
fn across<'a>(choices: &[(String, String)], side: Side, ink: Ink) -> Element<'a, Message> {
    let places = choices
        .iter()
        .map(|(glyph, name)| offered(glyph, name, ink))
        .chain((choices.len()..ACROSS).map(|_| Space::new().width(Length::FillPortion(1)).into()));

    Row::with_children(places)
        .width(Length::Fill)
        .height(Length::Fixed(line(ink)))
        .align_y(match side {
            Side::Trailing => Alignment::End,
            Side::Leading | Side::Middle => Alignment::Start
        })
        .into()
}

/// One choice: its glyph, and the name the desktop calls it by under it.
fn offered<'a>(glyph: &str, name: &str, ink: Ink) -> Element<'a, Message> {
    Column::with_children([
        text(glyph.to_owned())
            .size(ink.size * GLYPH)
            .color(ink.value)
            .into(),
        text(name.to_lowercase())
            .size(ink.size * NAME)
            .color(ink.label())
            .center()
            .into()
    ])
    .spacing(ink.size * UNDER)
    .width(Length::FillPortion(1))
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
            size:  10.0
        }
    }

    /// A choice is the subject of its block, not one more line in it.
    #[test]
    fn a_choice_is_drawn_larger_than_the_writing_around_it() {
        const { assert!(GLYPH > 1.0) };
        const { assert!(NAME < 1.0, "the name is a caption, not a reading") };
    }

    /// The block reserves its room before the choices are drawn into it, so
    /// what is asked for has to be what a line actually stands in.
    #[test]
    fn one_line_of_choices_asks_for_the_room_of_one_line() {
        assert_eq!(room(ACROSS, ink()), line(ink()));
    }

    #[test]
    fn a_set_too_wide_for_one_line_asks_for_the_lines_it_takes() {
        assert!(room(ACROSS + 1, ink()) > room(ACROSS, ink()));
        assert_eq!(room(0, ink()), line(ink()), "an empty set still has a line");
    }
}
