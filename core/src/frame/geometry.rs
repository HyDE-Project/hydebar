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
//! use hydebar_proto::config::FrameConfig;
//!
//! let frame = FrameConfig {
//!     enabled: true,
//!     ..FrameConfig::default()
//! };
//! let pieces = frame_pieces(1920.0, 1080.0, &frame, Position::Top, 38.0);
//!
//! assert_eq!(pieces.len(), 3);
//! ```

use hydebar_proto::config::{FrameConfig, Position};

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
/// begin where the band ends and round their joint. An edge paints at its
/// own width when one is set, at the shared thickness else; a non-positive
/// width skips that edge, and a width wider than half the screen is
/// clamped — a frame must leave something to frame.
#[must_use]
pub fn frame_pieces(
    screen_w: f32,
    screen_h: f32,
    frame: &FrameConfig,
    dock: Position,
    bar_height: f32
) -> Vec<FramePiece> {
    let radius = frame.rounding.max(0.0);
    let wide = screen_w / 2.0;
    let high = screen_h / 2.0;
    let width = |edge: Option<f32>| edge.unwrap_or(frame.thickness);

    let top_w = width(frame.widths.top).clamp(0.0, high);
    let bottom_w = width(frame.widths.bottom).clamp(0.0, high);
    let left_w = width(frame.widths.left).clamp(0.0, wide);
    let right_w = width(frame.widths.right).clamp(0.0, wide);

    let top_drawn = frame.edges.top && top_w > 0.0 && !matches!(dock, Position::Top);
    let bottom_drawn = frame.edges.bottom && bottom_w > 0.0 && !matches!(dock, Position::Bottom);

    let mut pieces = Vec::with_capacity(4);

    if top_drawn {
        pieces.push(FramePiece {
            x:            0.0,
            y:            0.0,
            w:            screen_w,
            h:            top_w,
            top_left:     radius,
            top_right:    radius,
            bottom_right: 0.0,
            bottom_left:  0.0
        });
    }

    if bottom_drawn {
        pieces.push(FramePiece {
            x:            0.0,
            y:            screen_h - bottom_w,
            w:            screen_w,
            h:            bottom_w,
            top_left:     0.0,
            top_right:    0.0,
            bottom_right: radius,
            bottom_left:  radius
        });
    }

    let top_gap = if top_drawn { top_w } else { 0.0 };
    let bottom_gap = if bottom_drawn { bottom_w } else { 0.0 };

    if frame.edges.left && left_w > 0.0 {
        VerticalRun {
            screen_h,
            strip: left_w,
            radius,
            dock,
            bar_height,
            top_gap,
            bottom_gap
        }
        .push(&mut pieces, 0.0, true);
    }

    if frame.edges.right && right_w > 0.0 {
        VerticalRun {
            screen_h,
            strip: right_w,
            radius,
            dock,
            bar_height,
            top_gap,
            bottom_gap
        }
        .push(&mut pieces, screen_w - right_w, false);
    }

    pieces
}

/// Shared context of one vertical strip: its width and the gaps above and
/// below it left by the bar band and the horizontal strips.
struct VerticalRun {
    screen_h:   f32,
    strip:      f32,
    radius:     f32,
    dock:       Position,
    bar_height: f32,
    top_gap:    f32,
    bottom_gap: f32
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
            self.dock,
            self.bar_height,
            self.top_gap,
            self.bottom_gap
        );

        if bottom <= top {
            return;
        }

        let joint_top = matches!(self.dock, Position::Top) && self.bar_height > 0.0;
        let joint_bottom = matches!(self.dock, Position::Bottom) && self.bar_height > 0.0;
        let corner_top = !joint_top && self.top_gap <= 0.0 && top <= 0.0;
        let corner_bottom = !joint_bottom && self.bottom_gap <= 0.0 && bottom >= self.screen_h;
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

    use hydebar_proto::config::{FrameConfig, FrameEdges, FrameWidths};

    use super::*;
    use crate::frame::vertical;

    const fn frame(thickness: f32) -> FrameConfig {
        FrameConfig {
            enabled: true,
            thickness,
            rounding: 25.0,
            edges: FrameEdges {
                top:    true,
                bottom: true,
                left:   true,
                right:  true
            },
            widths: FrameWidths {
                top:    None,
                bottom: None,
                left:   None,
                right:  None
            }
        }
    }

    #[test]
    fn a_top_bar_stands_on_three_strips() {
        let pieces = frame_pieces(1920.0, 1080.0, &frame(10.0), Position::Top, 38.0);

        assert_eq!(pieces.len(), 3);
        assert!(pieces.iter().all(|piece| piece.w > 0.0 && piece.h > 0.0));
    }

    #[test]
    fn a_lone_left_edge_rounds_its_joint_with_the_bar() {
        let mut config = frame(10.0);

        config.edges.top = false;
        config.edges.bottom = false;
        config.edges.right = false;

        let pieces = frame_pieces(1920.0, 1080.0, &config, Position::Top, 38.0);

        assert_eq!(pieces.len(), 1);

        let left = pieces[0];

        assert_eq!((left.x, left.y, left.w), (0.0, 38.0, 10.0));
        assert_eq!(left.h, 1080.0 - 38.0);
        assert_eq!((left.top_left, left.top_right), (25.0, 25.0));
        assert_eq!((left.bottom_left, left.bottom_right), (25.0, 0.0));
    }

    #[test]
    fn a_bottom_bar_leaves_two_strips_and_a_bottom_joint() {
        let pieces = frame_pieces(1920.0, 1080.0, &frame(10.0), Position::Bottom, 38.0);

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
        let pieces = frame_pieces(1920.0, 1080.0, &frame(10.0), Position::Top, 0.0);

        let left = pieces
            .iter()
            .find(|piece| piece.x == 0.0 && piece.w == 10.0)
            .expect("left strip");

        assert_eq!((left.top_left, left.top_right), (25.0, 0.0));
        assert_eq!((left.bottom_left, left.bottom_right), (0.0, 0.0));
    }

    #[test]
    fn no_thickness_paints_nothing() {
        assert!(frame_pieces(1920.0, 1080.0, &frame(0.0), Position::Top, 38.0).is_empty());
        assert!(frame_pieces(1920.0, 1080.0, &frame(-4.0), Position::Top, 38.0).is_empty());
    }

    #[test]
    fn a_bar_taller_than_the_screen_leaves_no_side_strips() {
        let pieces = frame_pieces(1920.0, 1080.0, &frame(10.0), Position::Top, 2000.0);

        assert!(pieces.iter().all(|piece| piece.w == 1920.0));
    }

    #[test]
    fn a_left_width_overrides_the_shared_thickness() {
        let mut config = frame(10.0);

        config.widths.left = Some(14.0);

        let pieces = frame_pieces(1920.0, 1080.0, &config, Position::Top, 38.0);
        let left = pieces
            .iter()
            .find(|piece| piece.x == 0.0 && piece.h > 100.0)
            .expect("left strip");

        assert_eq!(left.w, 14.0);

        let right = pieces
            .iter()
            .find(|piece| piece.x > 100.0)
            .expect("right strip");

        assert_eq!(right.w, 10.0);
    }

    #[test]
    fn vertical_ends_follow_the_drawn_gaps() {
        assert_eq!(
            vertical::vertical_end(1080.0, Position::Top, 38.0, 0.0, 10.0),
            (38.0, 1070.0)
        );
        assert_eq!(
            vertical::vertical_end(1080.0, Position::Bottom, 38.0, 10.0, 0.0),
            (10.0, 1042.0)
        );
    }
}
