//! The outline that leaves the strip with a block and becomes its border.
//!
//! Four things happen to one thin line, in order. It appears around the module
//! where the strip drew it. It travels the way the block travels, drawing the
//! line it came along behind it. On arrival it takes the shape of the area the
//! block is opening into and holds it as a border. And while the block writes
//! itself out, a brighter length of that border runs once around it and goes
//! out with it.
//!
//! Drawn under the blocks and off the very function they fly by, so the line
//! cannot describe a path the block did not take.

mod runner;
mod streak;

use hydebar_core::components::flip::offset_of;
use iced::{
    Color, Element, Length, Point, Rectangle, Renderer, Size, Theme,
    mouse::Cursor,
    widget::canvas::{self, Canvas, Frame as Sheet, Geometry, Path, Stroke}
};

use super::super::super::state::Message;

/// How wide the outline is drawn, in body letters.
const WIDTH: f32 = 0.09;

/// Radius the outline's corners are turned by, in body letters.
const CORNER: f32 = 0.3;

/// Share of the journey the outline spends as the island's own shape.
///
/// It keeps the module's shape while it is crossing and takes the shape of
/// what it is opening into as it lands, so what arrives is already the border
/// of the area rather than a box that snaps to it.
const KEEPS_ITS_SHAPE: f32 = 0.55;

/// One block's way, as the outline needs to know it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Way {
    /// Where the block rests once it has landed.
    pub seat:   Rectangle,
    /// The place the layout gave it on the strip, its own size and all.
    pub from:   Rectangle,
    /// The row of the strip it set off from.
    pub from_y: f32,
    /// How far along its own journey it is.
    pub travel: f32,
    /// How far the block has opened, once it is down.
    pub bloom:  f32
}

impl Way {
    /// The outline as it stands at `progress` of the journey.
    ///
    /// The place comes off the same function the block flies by; the shape is
    /// the module's until it is nearly down and the block's by the time it is.
    fn outline(&self, progress: f32) -> Rectangle {
        let at = Point::new(self.seat.x, self.seat.y);
        let offset = offset_of(progress, true, Some(self.from.x), Some(self.from_y), at);
        let shape = ((progress - KEEPS_ITS_SHAPE) / (1.0 - KEEPS_ITS_SHAPE)).clamp(0.0, 1.0);

        Rectangle::new(
            Point::new(at.x + offset.x, at.y + offset.y),
            Size::new(
                (self.seat.width - self.from.width).mul_add(shape, self.from.width),
                (self.seat.height - self.from.height).mul_add(shape, self.from.height)
            )
        )
    }

    /// How plainly the outline is drawn at all.
    ///
    /// Full while it travels, thinning away as the block finishes opening:
    /// the border has said what it came to say by then, and a canvas keeping
    /// every border it was drawn with is a canvas of boxes.
    fn ink(&self) -> f32 {
        if self.travel < 1.0 {
            return 1.0;
        }

        1.0 - self.bloom.clamp(0.0, 1.0)
    }
}

/// Draws every outline of a screen, under everything standing on it.
pub(super) fn trails<'a>(ways: Vec<Way>, ink: Color, size: f32) -> Element<'a, Message> {
    Canvas::new(Wake {
        ways,
        ink,
        width: (size * WIDTH).max(1.0),
        corner: size * CORNER
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Every outline of a screen, drawn over the whole of it.
#[derive(Debug)]
struct Wake {
    /// The blocks and how far each of them has come.
    ways:   Vec<Way>,
    /// What the canvas is written in, which is what this is drawn in.
    ink:    Color,
    /// How wide the outline is drawn.
    width:  f32,
    /// Radius the outline's corners are turned by.
    corner: f32
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
            let ink = way.ink();

            if ink <= 0.0 {
                continue;
            }

            let outline = way.outline(way.travel);

            streak::draw(&mut sheet, way, self.ink, self.width);
            self.border(&mut sheet, outline, ink);
            runner::draw(&mut sheet, outline, way, self.ink, self.width);
        }

        vec![sheet.into_geometry()]
    }
}

impl Wake {
    /// Draws the outline itself, wherever it stands right now.
    fn border(&self, sheet: &mut Sheet, outline: Rectangle, ink: f32) {
        sheet.stroke(
            &Path::rounded_rectangle(
                Point::new(outline.x, outline.y),
                Size::new(outline.width, outline.height),
                self.corner.into()
            ),
            Stroke::default()
                .with_width(self.width)
                .with_color(self.ink.scale_alpha(0.55 * ink))
        );
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    fn way(travel: f32, bloom: f32) -> Way {
        Way {
            seat: Rectangle::new(Point::new(40.0, 600.0), Size::new(300.0, 80.0)),
            from: Rectangle::new(Point::new(900.0, 0.0), Size::new(120.0, 30.0)),
            from_y: 8.0,
            travel,
            bloom
        }
    }

    #[test]
    fn the_outline_begins_around_the_place_the_strip_gave_the_module() {
        let outline = way(0.0, 0.0).outline(0.0);

        assert_eq!(outline.x, 900.0, "on the module, not on the block");
        assert_eq!(outline.y, 8.0);
        assert_eq!(outline.width, 120.0, "and in the module's own shape");
        assert_eq!(outline.height, 30.0);
    }

    #[test]
    fn the_outline_ends_as_the_border_of_the_area_that_opened() {
        let outline = way(1.0, 0.0).outline(1.0);
        let seat = way(1.0, 0.0).seat;

        assert_eq!(outline.x, seat.x);
        assert_eq!(outline.y, seat.y);
        assert_eq!(outline.width, seat.width);
        assert_eq!(outline.height, seat.height);
    }

    #[test]
    fn the_outline_keeps_the_modules_shape_for_the_first_half_of_the_way() {
        let outline = way(0.4, 0.0).outline(0.4);

        assert_eq!(outline.width, 120.0);
        assert_eq!(outline.height, 30.0);
    }

    #[test]
    fn the_border_goes_out_as_the_block_finishes_opening() {
        assert_eq!(way(0.5, 0.0).ink(), 1.0, "plain while it travels");
        assert_eq!(way(1.0, 0.0).ink(), 1.0, "and as it lands");
        assert!(way(1.0, 0.5).ink() < 1.0, "thinning as the block opens");
        assert_eq!(way(1.0, 1.0).ink(), 0.0, "gone once it is open");
    }
}
