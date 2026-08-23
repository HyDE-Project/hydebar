//! The row of glyphs a button module opens into.
//!
//! Some modules are not readings at all. A power button knows nothing and
//! reports nothing; what it has is a menu, and what the canvas can show of it
//! is the choices that menu offers. They stand in one row of glyphs — the
//! shape a set of choices takes everywhere — rather than unfolding the way
//! the pictures do: these are one press apart from one another, and a shape
//! that made one of them the one in force would be saying something the menu
//! does not say.

use iced::{
    Alignment, Element, Length,
    widget::{Row, container, text}
};

use super::{super::super::super::state::Message, Ink, Side};

/// How tall the row stands, as a share of the body ink.
const HEIGHT: f32 = 2.2;

/// How big a glyph is drawn, as a share of the body ink.
const GLYPH: f32 = 1.15;

/// How plainly the glyphs are drawn.
///
/// A step back from a reading: these are what the module could do rather than
/// anything it has to say, and a row of choices as loud as the figures beside
/// it would be read as the figures.
const QUIET: f32 = 0.75;

/// The room a row of choices takes, at the given ink.
pub(super) fn room(ink: Ink) -> f32 {
    ink.size * HEIGHT
}

/// Draws the choices in one row, lined up with the column they stand in.
pub(super) fn choices<'a>(glyphs: &[String], side: Side, ink: Ink) -> Element<'a, Message> {
    let row = Row::with_children(glyphs.iter().map(|glyph| drawn(glyph, ink)))
        .spacing(ink.size * 0.55)
        .height(Length::Fixed(room(ink)))
        .align_y(Alignment::Center);

    container(row)
        .width(Length::Fill)
        .align_x(side.alignment_x())
        .into()
}

/// One choice, drawn as the glyph the desktop's menu names it with.
fn drawn<'a>(glyph: &str, ink: Ink) -> Element<'a, Message> {
    text(glyph.to_owned())
        .size(ink.size * GLYPH)
        .color(ink.value.scale_alpha(QUIET))
        .into()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    /// The row is asked for its room before it is drawn, so the two agree.
    #[test]
    fn the_room_a_row_asks_for_is_the_room_it_stands_in() {
        let ink = Ink {
            value: iced::Color::WHITE,
            size:  10.0
        };

        assert_eq!(room(ink), 22.0);
    }
}
