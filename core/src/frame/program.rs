//! Canvas program painting the frame pieces.
//!
//! The program owns no geometry of its own: the pieces arrive ready to fill,
//! computed by [`super::geometry`], and the colour rides the bar palette the
//! way every other surface does. One rounded rectangle per piece, all in the
//! same fill — the pieces never overlap, so no seam can show.
//!
//! ```rust
//! use hydebar_core::{
//!     config::Position,
//!     frame::{geometry::frame_pieces, program::FrameCanvas}
//! };
//! use hydebar_proto::config::FrameEdges;
//! use iced::Color;
//!
//! let pieces = frame_pieces(
//!     1920.0,
//!     1080.0,
//!     10.0,
//!     25.0,
//!     FrameEdges::default(),
//!     Position::Top,
//!     38.0
//! );
//! let canvas = FrameCanvas {
//!     pieces,
//!     color: Color::BLACK
//! };
//!
//! assert_eq!(canvas.pieces.len(), 3);
//! ```

use iced::{
    Color, Point, Size,
    border::Radius,
    widget::canvas::{Geometry, Path, Program}
};

use super::geometry::FramePiece;

/// Paints the frame: one rounded rectangle per piece, in one colour.
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

        let cache = Cache::new();
        let pieces = self.pieces.clone();
        let color = self.color;

        vec![cache.draw(renderer, bounds.size(), |frame| {
            for piece in &pieces {
                let path = Path::rounded_rectangle(
                    Point::new(piece.x, piece.y),
                    Size::new(piece.w, piece.h),
                    Radius {
                        top_left:     piece.top_left,
                        top_right:    piece.top_right,
                        bottom_right: piece.bottom_right,
                        bottom_left:  piece.bottom_left
                    }
                );

                frame.fill(&path, color);
            }
        })]
    }
}
