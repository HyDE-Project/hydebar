//! Canvas program painting the frame pieces.
//!
//! The program owns no geometry of its own: the pieces arrive ready to fill,
//! computed by [`super::geometry`], and the colour rides the bar palette the
//! way every other surface does. Every piece is a subpath of one shape
//! filled under the even-odd rule — the pieces tile without overlap, so one
//! fill paints the whole ring and no seam can show where two pieces meet,
//! whatever the colour's transparency.
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
//! let pieces = frame_pieces(1920.0, 1080.0, &frame, Position::Top, 38.0);
//! let canvas = FrameCanvas {
//!     pieces,
//!     color: Color::BLACK
//! };
//!
//! assert_eq!(canvas.pieces.len(), 3);
//! ```

use iced::{
    Color, Point,
    border::Radius,
    widget::canvas::{Frame, Geometry, Program, fill, path}
};

use super::geometry::FramePiece;

/// Paints the frame: one even-odd path over the pieces, in one colour.
#[derive(Debug, Clone)]
pub struct FrameCanvas {
    /// Pieces to fill, computed by [`super::geometry::frame_pieces`].
    pub pieces: Vec<FramePiece>,
    /// Colour of the frame, from the bar palette.
    pub color:  Color
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

        let pieces = self.pieces.clone();
        let color = self.color;

        vec![Cache::new().draw(renderer, bounds.size(), |frame: &mut Frame| {
            let mut builder = path::Builder::new();

            for piece in &pieces {
                rounded_subpath(
                    &mut builder,
                    piece.x,
                    piece.y,
                    piece.w,
                    piece.h,
                    Radius {
                        top_left:     piece.top_left,
                        top_right:    piece.top_right,
                        bottom_right: piece.bottom_right,
                        bottom_left:  piece.bottom_left
                    }
                );
            }

            let shape = builder.build();

            frame.fill(
                &shape,
                fill::Fill {
                    style: color.into(),
                    rule:  fill::Rule::EvenOdd
                }
            );
        })]
    }
}

/// Traces one rounded rectangle into the builder.
///
/// A corner's radius is clamped to the lengths of the two edges it sweeps —
/// not to half the piece, which would halve a strip end's rounding: a
/// 10-pixel strip with a 10-pixel radius rounds its end over the full
/// thickness, the way a `Caelestia` joint does. Where both corners of one
/// side round past the width between them, the pair shares what is left.
fn rounded_subpath(builder: &mut path::Builder, x: f32, y: f32, w: f32, h: f32, radius: Radius) {
    let mut top_left = radius.top_left.max(0.0);
    let mut top_right = radius.top_right.max(0.0);
    let mut bottom_right = radius.bottom_right.max(0.0);
    let mut bottom_left = radius.bottom_left.max(0.0);

    let pair = |a: &mut f32, b: &mut f32, length: f32| {
        if *a + *b > length {
            let scale = length / (*a + *b);

            *a *= scale;
            *b *= scale;
        }
    };

    pair(&mut top_left, &mut top_right, w);
    pair(&mut bottom_left, &mut bottom_right, w);
    pair(&mut top_left, &mut bottom_left, h);
    pair(&mut top_right, &mut bottom_right, h);

    builder.move_to(Point::new(x + top_left, y));
    builder.line_to(Point::new(x + w - top_right, y));
    builder.arc_to(Point::new(x + w, y), Point::new(x + w, y + top_right), top_right);
    builder.line_to(Point::new(x + w, y + h - bottom_right));
    builder.arc_to(
        Point::new(x + w, y + h),
        Point::new(x + w - bottom_right, y + h),
        bottom_right
    );
    builder.line_to(Point::new(x + bottom_left, y + h));
    builder.arc_to(Point::new(x, y + h), Point::new(x, y + h - bottom_left), bottom_left);
    builder.line_to(Point::new(x, y + top_left));
    builder.arc_to(Point::new(x, y), Point::new(x + top_left, y), top_left);
    builder.close();
}
