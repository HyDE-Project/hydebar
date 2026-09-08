//! Canvas program painting the frame shape.
//!
//! The program owns no geometry of its own: the outline arrives ready to
//! fill, computed by [`super::geometry`], and the colour rides the bar
//! palette the way every other surface does. The ring is one even-odd path:
//! the square outer screen rect with the rounded inner hole cut out of it —
//! two subpaths of one fill, so no seam can show whatever the colour's
//! transparency, and the aperture rounds like the `Caelestia` border.
//!
//! ```rust
//! use hydebar_core::{
//!     config::Position,
//!     frame::{geometry::frame_pieces, program::FrameCanvas}
//! };
//! use hydebar_proto::config::FrameConfig;
//! use iced::Color;
//!
//! let frame = FrameConfig {
//!     enabled: true,
//!     ..FrameConfig::default()
//! };
//! let shape = frame_pieces(1920.0, 1080.0, &frame, Position::Top, 38.0);
//! let canvas = FrameCanvas {
//!     shape,
//!     color: Color::BLACK
//! };
//!
//! assert_eq!(canvas.shape.inner.w, 1900.0);
//! ```

use iced::{
    Color, Point,
    widget::canvas::{Frame, Geometry, Program, fill, path}
};

use super::geometry::{FramePiece, FrameShape};

/// Paints the frame: one even-odd path over the outer rect and the inner
/// hole.
#[derive(Debug, Clone)]
pub struct FrameCanvas {
    /// The outline to fill, computed by [`super::geometry::frame_pieces`].
    pub shape: FrameShape,
    /// Colour of the frame, from the bar palette.
    pub color: Color
}

impl<Message> Program<Message> for FrameCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor
    ) -> Vec<Geometry> {
        use iced::widget::canvas::Cache;

        let shape = self.shape;
        let color = self.color;

        vec![
            Cache::new().draw(renderer, bounds.size(), |frame: &mut Frame| {
                let mut builder = path::Builder::new();

                piece_subpath(&mut builder, shape.outer);
                piece_subpath(&mut builder, shape.inner);

                let path = builder.build();

                frame.fill(
                    &path,
                    fill::Fill {
                        style: color.into(),
                        rule:  fill::Rule::EvenOdd
                    }
                );
            }),
        ]
    }
}

/// Traces one rectangle into the builder.
///
/// A corner's radius is clamped to the lengths of the two edges it sweeps —
/// not to half the rect's width alone; where the edges are short the path
/// stays square. A fully square rect (the outer screen bound) traces
/// straight.
fn piece_subpath(builder: &mut path::Builder, piece: FramePiece) {
    let FramePiece {
        x,
        y,
        w,
        h,
        top_left,
        top_right,
        bottom_right,
        bottom_left
    } = piece;

    let pair = |a: f32, b: f32, length: f32| {
        let total = a.max(0.0) + b.max(0.0);

        if total > length && total > 0.0 {
            (a * length / total, b * length / total)
        } else {
            (a.max(0.0), b.max(0.0))
        }
    };

    let (tl, tr) = pair(top_left, top_right, w);
    let (bl, br) = pair(bottom_left, bottom_right, w);
    let (tl, bl) = pair(tl, bl, h);
    let (tr, br) = pair(tr, br, h);

    builder.move_to(Point::new(x + tl, y));
    builder.line_to(Point::new(x + w - tr, y));
    builder.arc_to(Point::new(x + w, y), Point::new(x + w, y + tr), tr);
    builder.line_to(Point::new(x + w, y + h - br));
    builder.arc_to(Point::new(x + w, y + h), Point::new(x + w - br, y + h), br);
    builder.line_to(Point::new(x + bl, y + h));
    builder.arc_to(Point::new(x, y + h), Point::new(x, y + h - bl), bl);
    builder.line_to(Point::new(x, y + tl));
    builder.arc_to(Point::new(x, y), Point::new(x + tl, y), tl);
    builder.close();
}
