//! Disjoint painted pieces of the frame.
//!
//! The frame is four edge strips that never overlap: the horizontal strips
//! span the full width, the vertical strips run between them — or between
//! the bar band and the far edge when the bar is docked to that side, see
//! [`super::vertical`]. A strip ending at a screen corner rounds its outer
//! corner, a strip meeting the bar band rounds both corners into a joint,
//! and a strip meeting another strip stays square so the two read as one
//! continuous run. The edge the bar is docked to never paints its straight
//! run: the bar band covers it, and a translucent bar would pick up a second
//! layer of colour from a strip hiding underneath.
//!
//! ```rust
//! use hydebar_core::{config::Position, frame::geometry::frame_pieces};
//! use hydebar_proto::config::FrameEdges;
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
//!
//! assert_eq!(pieces.len(), 3);
//! ```

use hydebar_proto::config::{FrameEdges, Position};

/// One painted rectangle of the frame, with a radius per corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePiece {
    /// Left edge of the piece, in screen pixels.
    pub x:            f32,
    /// Top edge of the piece, in screen pixels.
    pub y:            f32,
    /// Width of the piece, in screen pixels.
    pub w:            f32,
    /// Height of the piece, in screen pixels.
    pub h:            f32,
    /// Radius of the top left corner, in pixels.
    pub top_left:     f32,
    /// Radius of the top right corner, in pixels.
    pub top_right:    f32,
    /// Radius of the bottom right corner, in pixels.
    pub bottom_right: f32,
    /// Radius of the bottom left corner, in pixels.
    pub bottom_left:  f32
}

/// Painted pieces of the frame for a screen.
///
/// `bar_height` is the band the docked bar reserves: the strips beside it
/// begin where the band ends and round their joint. A non-positive thickness
/// paints nothing, and a thickness wider than half the screen is clamped —
/// a frame must leave something to frame.
#[must_use]
pub fn frame_pieces(
    screen_w: f32,
    screen_h: f32,
    thickness: f32,
    rounding: f32,
    edges: FrameEdges,
    dock: Position,
    bar_height: f32
) -> Vec<FramePiece> {
    let capped = screen_w.min(screen_h) / 2.0;
    let strip = thickness.clamp(0.0, capped);
    let radius = rounding.max(0.0);

    if strip <= 0.0 {
        return Vec::new();
    }

    let top_drawn = edges.top && !matches!(dock, Position::Top);
    let bottom_drawn = edges.bottom && !matches!(dock, Position::Bottom);

    let mut pieces = Vec::with_capacity(4);

    if top_drawn {
        pieces.push(FramePiece {
            x:            0.0,
            y:            0.0,
            w:            screen_w,
            h:            strip,
            top_left:     radius,
            top_right:    radius,
            bottom_right: 0.0,
            bottom_left:  0.0
        });
    }

    if bottom_drawn {
        pieces.push(FramePiece {
            x:            0.0,
            y:            screen_h - strip,
            w:            screen_w,
            h:            strip,
            top_left:     0.0,
            top_right:    0.0,
            bottom_right: radius,
            bottom_left:  radius
        });
    }

    let run = VerticalRun {
        screen_h,
        strip,
        radius,
        dock,
        bar_height,
        top_drawn,
        bottom_drawn
    };

    if edges.left {
        run.push(&mut pieces, 0.0, true);
    }

    if edges.right {
        run.push(&mut pieces, screen_w - strip, false);
    }

    pieces
}

/// Shared context of the two vertical strips of one screen.
struct VerticalRun {
    screen_h:     f32,
    strip:        f32,
    radius:       f32,
    dock:         Position,
    bar_height:   f32,
    top_drawn:    bool,
    bottom_drawn: bool
}

impl VerticalRun {
    /// Appends one vertical strip at `x`, unless the bar band leaves no room.
    ///
    /// `is_left` picks which side is outer: the left strip rounds its left
    /// corners at a screen corner, the right strip its right ones. A strip
    /// starting at the bar band rounds both top corners into the joint; a
    /// strip starting below a drawn horizontal strip stays square into it.
    fn push(&self, pieces: &mut Vec<FramePiece>, x: f32, is_left: bool) {
        let (top, bottom) = super::vertical::vertical_end(
            self.screen_h,
            self.strip,
            self.dock,
            self.bar_height,
            self.top_drawn,
            self.bottom_drawn
        );

        if bottom <= top {
            return;
        }

        let joint_top = matches!(self.dock, Position::Top) && self.bar_height > 0.0;
        let joint_bottom = matches!(self.dock, Position::Bottom) && self.bar_height > 0.0;
        let corner_top = !joint_top && !self.top_drawn && top <= 0.0;
        let corner_bottom = !joint_bottom && !self.bottom_drawn && bottom >= self.screen_h;

        let (outer_top, inner_top) =
            super::vertical::end_radii(joint_top, corner_top, self.radius);
        let (outer_bottom, inner_bottom) =
            super::vertical::end_radii(joint_bottom, corner_bottom, self.radius);

        pieces.push(if is_left {
            FramePiece {
                x,
                y: top,
                w: self.strip,
                h: bottom - top,
                top_left: outer_top,
                top_right: inner_top,
                bottom_right: inner_bottom,
                bottom_left: outer_bottom
            }
        } else {
            FramePiece {
                x,
                y: top,
                w: self.strip,
                h: bottom - top,
                top_left: inner_top,
                top_right: outer_top,
                bottom_right: outer_bottom,
                bottom_left: inner_bottom
            }
        });
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    const EDGES: FrameEdges = FrameEdges {
        top:    true,
        bottom: true,
        left:   true,
        right:  true
    };

    #[test]
    fn a_top_bar_stands_on_three_strips() {
        let pieces = frame_pieces(1920.0, 1080.0, 10.0, 25.0, EDGES, Position::Top, 38.0);

        assert_eq!(pieces.len(), 3);
        assert!(pieces.iter().all(|piece| piece.w > 0.0 && piece.h > 0.0));
    }

    #[test]
    fn a_lone_left_edge_rounds_its_joint_with_the_bar() {
        let edges = FrameEdges {
            top:    false,
            bottom: false,
            left:   true,
            right:  false
        };
        let pieces = frame_pieces(1920.0, 1080.0, 10.0, 25.0, edges, Position::Top, 38.0);

        assert_eq!(pieces.len(), 1);

        let left = pieces[0];

        assert_eq!((left.x, left.y, left.w), (0.0, 38.0, 10.0));
        assert_eq!(left.h, 1080.0 - 38.0);
        assert_eq!((left.top_left, left.top_right), (25.0, 25.0));
        assert_eq!((left.bottom_left, left.bottom_right), (25.0, 0.0));
    }

    #[test]
    fn a_bottom_bar_leaves_two_strips_and_a_bottom_joint() {
        let pieces = frame_pieces(1920.0, 1080.0, 10.0, 25.0, EDGES, Position::Bottom, 38.0);

        assert_eq!(pieces.len(), 3);

        let left = pieces
            .iter()
            .find(|piece| piece.x == 0.0 && piece.w == 10.0)
            .expect("left strip");

        assert_eq!(left.y, 10.0);
        assert_eq!((left.top_left, left.top_right), (0.0, 0.0));
        assert_eq!((left.bottom_left, left.bottom_right), (25.0, 25.0));
    }

    #[test]
    fn a_side_strip_meets_the_far_strip_square() {
        let pieces = frame_pieces(1920.0, 1080.0, 10.0, 25.0, EDGES, Position::Top, 0.0);

        let left = pieces
            .iter()
            .find(|piece| piece.x == 0.0 && piece.w == 10.0)
            .expect("left strip");

        assert_eq!((left.top_left, left.top_right), (25.0, 0.0));
        assert_eq!((left.bottom_left, left.bottom_right), (0.0, 0.0));
    }

    #[test]
    fn no_thickness_paints_nothing() {
        assert!(frame_pieces(1920.0, 1080.0, 0.0, 25.0, EDGES, Position::Top, 38.0).is_empty());
        assert!(frame_pieces(1920.0, 1080.0, -4.0, 25.0, EDGES, Position::Top, 38.0).is_empty());
    }

    #[test]
    fn a_bar_taller_than_the_screen_leaves_no_side_strips() {
        let pieces = frame_pieces(1920.0, 1080.0, 10.0, 25.0, EDGES, Position::Top, 2000.0);

        assert!(pieces.iter().all(|piece| piece.w == 1920.0));
    }
}
