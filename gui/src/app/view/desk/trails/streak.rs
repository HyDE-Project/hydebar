//! The line the outline draws behind it on the way across.
//!
//! Not a wake for its own sake: it is what says the block came from the strip
//! rather than appeared where it stands. Strongest at the outline and gone
//! where the module was, so the eye reads it from the block backwards.

use iced::{
    Color, Point,
    widget::canvas::{Frame as Sheet, Path, Stroke}
};

use super::Way;

/// How many steps of the way the line is drawn in.
///
/// Enough that the bend where the block stops moving sideways and comes down
/// is a curve rather than a corner, few enough that a screen of them is a
/// handful of lines.
const STEPS: usize = 24;

/// How plainly the line is drawn where it is strongest.
const INK: f32 = 0.35;

/// Draws the line one outline came along, thinning and fading behind it.
///
/// Stepped rather than stroked in one go because a stroke carries one width
/// and one colour, and the whole point of this line is that it has neither.
pub(super) fn draw(sheet: &mut Sheet, way: &Way, ink: Color, width: f32) {
    if way.travel <= 0.0 || way.travel >= 1.0 {
        return;
    }

    let along = walked(way);

    for step in 1..along.len() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a line is drawn in a few dozen steps"
        )]
        let share = step as f32 / (along.len() - 1) as f32;

        sheet.stroke(
            &Path::line(along[step - 1], along[step]),
            Stroke::default()
                .with_width((width * share).max(0.5))
                .with_color(ink.scale_alpha(INK * share * share))
        );
    }
}

/// The places the outline's middle stood on, oldest first.
fn walked(way: &Way) -> Vec<Point> {
    (0..=STEPS)
        .map(|step| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a line is drawn in a few dozen steps"
            )]
            let progress = way.travel * (step as f32 / STEPS as f32);
            let outline = way.outline(progress);

            Point::new(
                outline.x + outline.width / 2.0,
                outline.y + outline.height / 2.0
            )
        })
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::{Rectangle, Size};

    use super::*;

    fn way(travel: f32) -> Way {
        Way {
            seat: Rectangle::new(Point::new(40.0, 600.0), Size::new(300.0, 80.0)),
            from: Rectangle::new(Point::new(900.0, 0.0), Size::new(120.0, 30.0)),
            from_y: 8.0,
            travel,
            bloom: 0.0
        }
    }

    #[test]
    fn the_line_begins_in_the_middle_of_the_module_it_left() {
        let along = walked(&way(0.5));

        assert_eq!(along[0], Point::new(960.0, 23.0));
    }

    #[test]
    fn the_line_lengthens_as_the_outline_travels() {
        let span = |travel: f32| {
            let along = walked(&way(travel));
            let last = along[along.len() - 1];

            (last.x - along[0].x).hypot(last.y - along[0].y)
        };

        assert!(span(0.8) > span(0.2));
    }
}
