//! The fading streak a block leaves along the way it came.
//!
//! A block that simply arrives has moved; a block that leaves a wake has
//! travelled, and the wake is what tells the eye where it came from once it
//! is standing still. Drawn under everything else so the blocks fly over
//! their own trails rather than through them, and drawn from the very
//! function the blocks fly by, so the streak cannot describe a path the block
//! did not take.

use hydebar_core::components::flip::offset_of;
use iced::{
    Color, Element, Length, Point, Rectangle, Renderer, Theme,
    mouse::Cursor,
    widget::canvas::{self, Canvas, Frame as Sheet, Geometry, Path, Stroke}
};

use super::super::super::state::Message;

/// How many steps of the way a trail is drawn in.
///
/// Enough that the bend where a block stops moving sideways and comes down is
/// a curve rather than a corner, few enough that a screen of them is a
/// handful of lines.
const STEPS: usize = 24;

/// How wide the streak is at the block, in body letters.
const WIDTH: f32 = 0.5;

/// How plainly the streak is drawn where it is strongest.
const INK: f32 = 0.3;

/// One block's way, as the trail needs to know it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Way {
    /// Where the block rests once it has landed.
    pub seat:   Rectangle,
    /// The place the layout gave it on the strip, its own size and all.
    pub from:   Rectangle,
    /// The row of the strip it set off from.
    pub from_y: f32,
    /// How far along its own journey it is.
    pub travel: f32
}

/// Draws every trail of a screen, under everything the canvas stands on it.
pub(super) fn trails<'a>(ways: Vec<Way>, ink: Color, size: f32) -> Element<'a, Message> {
    Canvas::new(Wake {
        ways,
        ink,
        width: size * WIDTH
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Every way of a screen, drawn over the whole of it.
#[derive(Debug)]
struct Wake {
    /// The blocks and how far each of them has come.
    ways:  Vec<Way>,
    /// What the canvas is written in, which is what this is drawn in.
    ink:   Color,
    /// How wide the streak is at the block.
    width: f32
}

impl canvas::Program<Message> for Wake {
    type State = ();

    fn draw(
        &self,
        (): &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor
    ) -> Vec<Geometry> {
        let mut sheet = Sheet::new(renderer, bounds.size());

        for way in &self.ways {
            self.streak(&mut sheet, way);
        }

        vec![sheet.into_geometry()]
    }
}

impl Wake {
    /// Draws the way one block came, thinning and fading behind it.
    ///
    /// Stepped rather than stroked in one line because a stroke carries one
    /// width and one colour, and the whole point of a wake is that it has
    /// neither: it is strongest at the block and gone where the block set off.
    fn streak(&self, sheet: &mut Sheet, way: &Way) {
        if way.travel <= 0.0 || way.travel >= 1.0 {
            return;
        }

        let along = walked(way);

        for step in 1..along.len() {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a trail is drawn in a few dozen steps"
            )]
            let share = step as f32 / (along.len() - 1) as f32;
            let fading = (1.0 - way.travel).min(1.0);

            sheet.stroke(
                &Path::line(along[step - 1], along[step]),
                Stroke::default()
                    .with_width((self.width * share).max(0.5))
                    .with_color(self.ink.scale_alpha(INK * share * share * fading))
            );
        }
    }
}

/// The places the block stood on, from where it set off to where it is now.
///
/// Traced through the middle of the block rather than its corner, and the
/// middle moves: on the strip the block is an island a few letters wide, on
/// the canvas it is a table of readings, and the point the eye follows is the
/// middle of whichever of the two it looks like at that moment. Taken from
/// the corner alone the streak began half a block away from the module it
/// belongs to, which is the one place it must not begin.
fn walked(way: &Way) -> Vec<Point> {
    let at = Point::new(way.seat.x, way.seat.y);

    (0..=STEPS)
        .map(|step| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a trail is drawn in a few dozen steps"
            )]
            let progress = way.travel * (step as f32 / STEPS as f32);
            let offset = offset_of(progress, true, Some(way.from.x), Some(way.from_y), at);
            let shape = grown(progress);
            let across = (way.seat.width - way.from.width).mul_add(shape, way.from.width);
            let down = (way.seat.height - way.from.height).mul_add(shape, way.from.height);

            Point::new(at.x + offset.x + across / 2.0, at.y + offset.y + down / 2.0)
        })
        .collect()
}

/// How much of the way from the strip's shape to the canvas's one is done.
const fn grown(progress: f32) -> f32 {
    progress.clamp(0.0, 1.0)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::Size;

    use super::*;

    fn way(travel: f32) -> Way {
        Way {
            seat: Rectangle::new(Point::new(40.0, 600.0), Size::new(300.0, 80.0)),
            from: Rectangle::new(Point::new(900.0, 0.0), Size::new(120.0, 30.0)),
            from_y: 8.0,
            travel
        }
    }

    #[test]
    fn the_way_begins_in_the_middle_of_the_place_the_strip_gave_the_module() {
        let along = walked(&way(0.5));
        let leaving = way(0.5).from;

        assert_eq!(
            along[0],
            Point::new(leaving.x + leaving.width / 2.0, 8.0 + leaving.height / 2.0),
            "the first step is the middle of the island, not a corner of the block"
        );
    }

    #[test]
    fn the_way_ends_in_the_middle_of_the_block_as_it_stands_now() {
        let travelling = way(0.5);
        let along = walked(&travelling);
        let last = along[along.len() - 1];
        let now = offset_of(
            0.5,
            true,
            Some(travelling.from.x),
            Some(8.0),
            Point::new(travelling.seat.x, travelling.seat.y)
        );
        let grown_to = |from: f32, to: f32| from + (to - from) * 0.5;

        assert!(
            (last.x
                - (travelling.seat.x
                    + now.x
                    + grown_to(travelling.from.width, travelling.seat.width) / 2.0))
                .abs()
                < 0.001
        );
        assert!(
            (last.y
                - (travelling.seat.y
                    + now.y
                    + grown_to(travelling.from.height, travelling.seat.height) / 2.0))
                .abs()
                < 0.001
        );
    }

    #[test]
    fn the_way_lengthens_as_the_block_travels() {
        let early = walked(&way(0.2));
        let late = walked(&way(0.8));

        let span = |along: &[Point]| {
            (along[along.len() - 1].x - along[0].x).hypot(along[along.len() - 1].y - along[0].y)
        };

        assert!(span(&late) > span(&early));
    }
}
