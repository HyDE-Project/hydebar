//! The reel of pictures, drawn with the one in force big in the middle.
//!
//! A theme is a look and a wallpaper is a picture; neither is a sentence, and
//! a block that spelled them out told the user the name of something they were
//! looking straight at. The desktop keeps a crop of every one of them, so the
//! block shows the pictures: the one in force at full height in the middle of
//! the row, its neighbours narrowing and dimming away from it to either side —
//! the shape every picker of pictures has ever taken.

use hydebar_core::modules::desk::looks::{REACH, Reel, Slide};
use iced::{
    Alignment, Element, Length,
    widget::{Row, Stack, container, image, text}
};

use super::{super::super::super::state::Message, Ink};

/// How tall the row stands, as a share of the body ink.
///
/// Six lines and a half: a picture at the height of two says nothing, and a
/// block that is mostly picture stops being a reading.
const HEIGHT: f32 = 6.5;

/// Share of the row's width the picture in force takes.
const IN_FORCE: u16 = 12;

/// Share of the row's width a picture takes at each remove from that one.
///
/// Halving outwards: the fall says which way the eye is meant to travel, and
/// the last one is still wide enough to read as a picture rather than a rule.
const AWAY: [u16; REACH] = [4, 2, 1];

/// How tall a picture stands at each remove, as a share of the row.
const SHORTER: [f32; REACH] = [0.86, 0.74, 0.64];

/// How plainly a picture out of force is drawn.
const DIMMED: f32 = 0.55;

/// The room a reel takes, at the given ink.
pub(super) fn room(ink: Ink) -> f32 {
    ink.size * HEIGHT
}

/// Draws the reel: the picture in force in the middle, the rest either side.
pub(super) fn accordion<'a>(reel: &Reel, ink: Ink) -> Element<'a, Message> {
    let tall = room(ink);
    let middle = reel
        .shown
        .iter()
        .position(|slide| slide.active)
        .unwrap_or_default();

    Row::with_children(reel.shown.iter().enumerate().map(|(index, slide)| {
        let away = index.abs_diff(middle);

        drawn(slide, share(away), tall * shorter(away), ink)
    }))
    .spacing(ink.size * 0.3)
    .width(Length::Fill)
    .height(Length::Fixed(tall))
    .align_y(Alignment::Center)
    .into()
}

/// The width one picture takes, `away` places from the one in force.
fn share(away: usize) -> u16 {
    if away == 0 {
        return IN_FORCE;
    }

    AWAY.get(away - 1).copied().unwrap_or(1)
}

/// How tall one picture stands, `away` places from the one in force.
fn shorter(away: usize) -> f32 {
    if away == 0 {
        return 1.0;
    }

    SHORTER.get(away - 1).copied().unwrap_or(SHORTER[REACH - 1])
}

/// Draws one picture of the reel, at the width and height its place gives it.
fn drawn<'a>(slide: &Slide, portion: u16, tall: f32, ink: Ink) -> Element<'a, Message> {
    let picture = image(slide.picture.clone())
        .width(Length::Fill)
        .height(Length::Fixed(tall))
        .content_fit(iced::ContentFit::Cover)
        .border_radius(ink.size * 0.3)
        .opacity(if slide.active { 1.0 } else { DIMMED });

    let shown: Element<'a, Message> = if slide.active {
        Stack::new()
            .push(picture)
            .push(named(&slide.name, ink))
            .into()
    } else {
        picture.into()
    };

    container(shown)
        .width(Length::FillPortion(portion.max(1)))
        .height(Length::Fixed(tall))
        .clip(true)
        .into()
}

/// The name of the picture in force, written across the foot of it.
///
/// On the picture rather than under the row: a caption under a picture this
/// narrow either wraps or is cut, and a name written on the thing it names is
/// what every picker does with the tile the user is standing on.
fn named<'a>(name: &str, ink: Ink) -> Element<'a, Message> {
    let written = container(
        text(name.to_owned())
            .size(ink.size * 0.85)
            .color(iced::Color::WHITE)
    )
    .width(Length::Fill)
    .padding(
        iced::Padding::default()
            .left(ink.size * 0.4)
            .right(ink.size * 0.4)
    )
    .style(|_| container::Style {
        background: Some(iced::Color::BLACK.scale_alpha(0.45).into()),
        ..container::Style::default()
    });

    container(written)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Alignment::End)
        .into()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    /// The whole point of the shape: the eye is meant to land on one picture.
    #[test]
    fn the_one_in_force_is_the_widest_and_the_tallest() {
        assert!(share(0) > share(1));
        assert!(shorter(0) > shorter(1));
    }

    #[test]
    fn the_pictures_narrow_and_shorten_away_from_it() {
        for away in 1..REACH {
            assert!(
                share(away) > share(away + 1),
                "a picture further out is not wider: {away}"
            );
            assert!(shorter(away) > shorter(away + 1));
        }
    }

    #[test]
    fn a_picture_past_the_reach_is_drawn_as_the_furthest_one() {
        assert_eq!(share(REACH + 3), 1);
        assert_eq!(shorter(REACH + 3), SHORTER[REACH - 1]);
    }
}
