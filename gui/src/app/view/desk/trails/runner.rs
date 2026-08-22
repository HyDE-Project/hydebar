//! The brighter length that runs once around a border while the block opens.
//!
//! One lap, no more. A light that keeps going is a spinner, and a spinner says
//! the bar is waiting for something; this says the opposite — the block is
//! writing itself out, here is the shape it will fill, and by the time the
//! light is back where it started the block is open and both are gone.

use iced::{
    Color, Point, Rectangle,
    widget::canvas::{Frame as Sheet, Path, Stroke}
};

use super::Way;

/// How much of the border the running length covers.
const LENGTH: f32 = 0.18;

/// How many steps the running length itself is drawn in.
///
/// It fades from its own head to its own tail, so it is stepped for the same
/// reason the line behind the outline is.
const STEPS: usize = 10;

/// How plainly the head of the running length is drawn.
const INK: f32 = 0.95;

/// Draws the length running around `outline`, at the lap the block is on.
pub(super) fn draw(sheet: &mut Sheet, outline: Rectangle, way: &Way, ink: Color, width: f32) {
    if way.travel < 1.0 || way.bloom <= 0.0 || way.bloom >= 1.0 {
        return;
    }

    let head = way.bloom.clamp(0.0, 1.0);
    let fading = 1.0 - head;

    for step in 0..STEPS {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the running length is drawn in a few steps"
        )]
        let back = step as f32 / STEPS as f32;
        #[expect(
            clippy::cast_precision_loss,
            reason = "the running length is drawn in a few steps"
        )]
        let further = (step + 1) as f32 / STEPS as f32;

        let from = around(outline, LENGTH.mul_add(-further, head));
        let to = around(outline, LENGTH.mul_add(-back, head));

        sheet.stroke(
            &Path::line(from, to),
            Stroke::default()
                .with_width(width * 1.4)
                .with_color(ink.scale_alpha(INK * (1.0 - back) * fading))
        );
    }
}

/// The point `along` of the way around a rectangle, clockwise from its top
/// left corner.
///
/// A lap before the start is a lap that has not begun: it is held at the
/// corner it starts from rather than wrapping round to the far side, so the
/// length grows out of the corner instead of arriving from nowhere.
fn around(outline: Rectangle, along: f32) -> Point {
    let along = along.clamp(0.0, 1.0);
    let across = outline.width;
    let down = outline.height;
    let perimeter = 2.0f32.mul_add(across + down, 0.0).max(f32::EPSILON);
    let walked = along * perimeter;

    if walked <= across {
        return Point::new(outline.x + walked, outline.y);
    }

    if walked <= across + down {
        return Point::new(outline.x + across, outline.y + (walked - across));
    }

    if walked <= 2.0f32.mul_add(across, down) {
        return Point::new(
            outline.x + across - (walked - across - down),
            outline.y + down
        );
    }

    Point::new(
        outline.x,
        outline.y + down - (walked - 2.0f32.mul_add(across, down))
    )
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::Size;

    use super::*;

    fn box_of() -> Rectangle {
        Rectangle::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0))
    }

    #[test]
    fn the_lap_starts_and_ends_at_the_same_corner() {
        assert_eq!(around(box_of(), 0.0), Point::new(10.0, 20.0));
        assert_eq!(around(box_of(), 1.0), Point::new(10.0, 20.0));
    }

    #[test]
    fn the_lap_runs_clockwise_through_all_four_corners() {
        let perimeter = 300.0_f32;

        assert_eq!(around(box_of(), 100.0 / perimeter), Point::new(110.0, 20.0));
        assert_eq!(around(box_of(), 150.0 / perimeter), Point::new(110.0, 70.0));
        assert_eq!(around(box_of(), 250.0 / perimeter), Point::new(10.0, 70.0));
    }

    #[test]
    fn a_lap_before_it_begins_is_held_at_the_corner() {
        assert_eq!(around(box_of(), -0.4), around(box_of(), 0.0));
    }
}
