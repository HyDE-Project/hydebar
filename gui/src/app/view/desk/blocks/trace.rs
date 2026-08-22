//! The last few minutes of a reading, drawn as the shape they make.
//!
//! A number says where a reading is; a trace says where it has been, and the
//! two together are what a monitor has always shown. Drawn filled rather than
//! as a bare line: a filled shape reads at a glance from across a room, which
//! is the distance this canvas is meant for.

use iced::{
    Element, Length, Point, Rectangle, Renderer, Theme,
    mouse::Cursor,
    widget::canvas::{self, Canvas, Frame as Sheet, Geometry, Path, Stroke}
};

use super::Ink;
use crate::app::Message;

/// How tall a trace stands, as a share of the body ink.
const HEIGHT: f32 = 2.6;

/// The room a trace takes, at the given ink.
pub(super) fn room(ink: Ink) -> f32 {
    ink.size * HEIGHT
}

/// Draws one trace across the whole width of the block.
pub(super) fn trace<'a>(readings: &[f32], ceiling: f32, ink: Ink) -> Element<'a, Message> {
    Canvas::new(Line {
        readings: readings.to_vec(),
        ceiling,
        ink
    })
    .width(Length::Fill)
    .height(Length::Fixed(room(ink)))
    .into()
}

/// One reading over time, drawn to whatever width the block has.
#[derive(Debug)]
struct Line {
    /// The readings, oldest first.
    readings: Vec<f32>,
    /// What the top of the drawing stands for.
    ceiling:  f32,
    /// What the column is written in, which is what this is drawn in.
    ink:      Ink
}

impl canvas::Program<Message> for Line {
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
        let points = self.points(bounds);

        if points.len() < 2 {
            return vec![sheet.into_geometry()];
        }

        let under = Path::new(|shape| {
            shape.move_to(Point::new(points[0].x, bounds.height));

            for point in &points {
                shape.line_to(*point);
            }

            shape.line_to(Point::new(points[points.len() - 1].x, bounds.height));
            shape.close();
        });

        sheet.fill(&under, self.ink.value.scale_alpha(0.16));

        let over = Path::new(|shape| {
            shape.move_to(points[0]);

            for point in points.iter().skip(1) {
                shape.line_to(*point);
            }
        });

        sheet.stroke(
            &over,
            Stroke::default()
                .with_width(1.5)
                .with_color(self.ink.value.scale_alpha(0.7))
        );

        vec![sheet.into_geometry()]
    }
}

impl Line {
    /// The readings placed on the sheet, oldest at the left.
    ///
    /// The ceiling is what the top of the drawing stands for, and a reading
    /// above it is drawn at the top rather than off the sheet: a temperature
    /// beyond what the scale was drawn for is still a temperature, and a
    /// trace that vanished would say the sensor had.
    fn points(&self, bounds: Rectangle) -> Vec<Point> {
        let ceiling = self.ceiling.max(f32::EPSILON);
        let last = self.readings.len().saturating_sub(1);

        if last == 0 {
            return Vec::new();
        }

        #[expect(
            clippy::cast_precision_loss,
            reason = "a trail holds a few dozen readings"
        )]
        let step = bounds.width / last as f32;

        self.readings
            .iter()
            .enumerate()
            .map(|(at, reading)| {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a trail holds a few dozen readings"
                )]
                let x = at as f32 * step;
                let share = (reading / ceiling).clamp(0.0, 1.0);

                Point::new(x, bounds.height * (1.0 - share))
            })
            .collect()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::{Color, Size};

    use super::*;

    fn ink() -> Ink {
        Ink {
            value: Color::WHITE,
            size:  14.0
        }
    }

    fn line(readings: &[f32], ceiling: f32) -> Line {
        Line {
            readings: readings.to_vec(),
            ceiling,
            ink: ink()
        }
    }

    fn sheet() -> Rectangle {
        Rectangle::new(Point::ORIGIN, Size::new(100.0, 20.0))
    }

    #[test]
    fn the_oldest_reading_stands_at_the_left_and_the_newest_at_the_right() {
        let points = line(&[0.0, 50.0, 100.0], 100.0).points(sheet());

        assert_eq!(points[0].x, 0.0);
        assert_eq!(points[2].x, 100.0);
    }

    #[test]
    fn a_full_reading_stands_at_the_top_and_an_empty_one_at_the_bottom() {
        let points = line(&[0.0, 100.0], 100.0).points(sheet());

        assert_eq!(points[0].y, 20.0);
        assert_eq!(points[1].y, 0.0);
    }

    #[test]
    fn a_reading_past_the_ceiling_stays_on_the_sheet() {
        let points = line(&[0.0, 250.0], 100.0).points(sheet());

        assert_eq!(points[1].y, 0.0, "drawn at the top, not off it");
    }

    #[test]
    fn a_single_reading_draws_nothing() {
        assert!(line(&[42.0], 100.0).points(sheet()).is_empty());
    }
}
