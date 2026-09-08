//! The ring around the screen: a square screen minus a rounded aperture.
//!
//! Like the `Caelestia` border, the frame is not four glued strips but one
//! shape: the untouched screen rectangle, less an inner rectangle inset by
//! each drawn edge. The screen boundary stays square — the physical corners
//! of the output are fully painted — and every rounding lives on the inner
//! aperture, where the content faces it. The docked bar band takes no cut of
//! its own: the hole already sits below the bar band, so the ring reads as
//! one paint travelling out of the strip and around the screen.
//!
//! ```rust
//! use hydebar_core::{config::Position, frame::geometry::frame_pieces};
//! use hydebar_proto::config::FrameConfig;
//!
//! let frame = FrameConfig {
//!     enabled: true,
//!     ..FrameConfig::default()
//! };
//! let shape = frame_pieces(1920.0, 1080.0, &frame, Position::Top, 38.0);
//!
//! assert_eq!(shape.inner.x, 10.0);
//! assert_eq!(shape.inner.bottom_left, 25.0);
//! ```

use hydebar_proto::config::{FrameConfig, FrameEdge, Position};

/// One rectangle of the frame shape: the outer screen rect or the content
/// hole carved out of it, each corner rounded as the shape says.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FramePiece {
    /// Left edge of the rectangle, in screen pixels.
    pub x:            f32,
    /// Top edge of the rectangle, in screen pixels.
    pub y:            f32,
    /// Width of the rectangle, in screen pixels.
    pub w:            f32,
    /// Height of the rectangle, in screen pixels.
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

/// The outline of the frame: the outer and inner rectangles.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameShape {
    /// The far bound of the ring: the full screen, square at the corners.
    pub outer: FramePiece,
    /// The hole carved out of it: the content area, rounded inward.
    pub inner: FramePiece
}

/// The frame's outline for a screen.
///
/// `bar_height` is the band the docked bar reserves: the content hole sits
/// below the band even when no strip is drawn there. Each drawn edge insets
/// the hole at its own width, the shared thickness or its own override; a
/// disabled edge insets nothing and a corner it gives way on stays square.
/// Every inner corner carries the configured rounding whole — the rounding
/// belongs to the aperture, not to the bezel, so it is never shaved down by
/// the thickness of a strip.
#[must_use]
pub fn frame_pieces(
    screen_w: f32,
    screen_h: f32,
    frame: &FrameConfig,
    dock: Position,
    bar_height: f32
) -> FrameShape {
    let radius = frame.rounding.max(0.0);
    let inset = |edge: &FrameEdge| {
        edge.enabled
            .then_some(edge.width.unwrap_or(frame.thickness))
            .filter(|width| *width > 0.0)
    };
    let band = matches!(dock, Position::Top | Position::Bottom);
    let band_height = if band { bar_height.max(0.0) } else { 0.0 };

    let mut top = inset(&frame.edges.top).unwrap_or(0.0);
    let mut bottom = inset(&frame.edges.bottom).unwrap_or(0.0);

    if matches!(dock, Position::Top) {
        top = top.max(band_height);
    }

    if matches!(dock, Position::Bottom) {
        bottom = bottom.max(band_height);
    }

    let left = inset(&frame.edges.left).unwrap_or(0.0);
    let right = inset(&frame.edges.right).unwrap_or(0.0);

    let outer = FramePiece {
        x: 0.0,
        y: 0.0,
        w: screen_w,
        h: screen_h,
        ..FramePiece::default()
    };

    let hole_w = (screen_w - left - right).clamp(0.0, screen_w.max(0.0));
    let hole_h = (screen_h - top - bottom).clamp(0.0, screen_h.max(0.0));
    let hole_clamp = (hole_w / 2.0).min(hole_h / 2.0).max(0.0);

    let inner = FramePiece {
        x:            left,
        y:            top,
        w:            hole_w,
        h:            hole_h,
        top_left:     radius_in(radius, left, top, hole_clamp),
        top_right:    radius_in(radius, right, top, hole_clamp),
        bottom_right: radius_in(radius, right, bottom, hole_clamp),
        bottom_left:  radius_in(radius, left, bottom, hole_clamp)
    };

    FrameShape {
        outer,
        inner
    }
}

/// Radius of an inner corner: the configured rounding, where the content
/// hole has two drawn sides to round between; a corner one side gives way
/// on stays square.
fn radius_in(radius: f32, one: f32, other: f32, clamp: f32) -> f32 {
    if one > 0.0 && other > 0.0 {
        radius.clamp(0.0, clamp)
    } else {
        0.0
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use hydebar_proto::config::{FrameEdge, FrameEdges};

    use super::*;

    fn frame(
        thickness: f32,
        rounding: f32,
        top: bool,
        bottom: bool,
        left: bool,
        right: bool
    ) -> FrameConfig {
        let edge = |enabled: bool| FrameEdge {
            enabled,
            width: None,
            entries: Vec::new()
        };

        FrameConfig {
            enabled: true,
            thickness,
            rounding,
            edges: FrameEdges {
                top:    edge(top),
                bottom: edge(bottom),
                left:   edge(left),
                right:  edge(right)
            }
        }
    }

    #[test]
    fn the_outer_stays_square_where_the_screen_meets_it() {
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, true, true, true, true),
            Position::Top,
            38.0
        );

        assert_eq!(
            shape.outer,
            FramePiece {
                x: 0.0,
                y: 0.0,
                w: 1920.0,
                h: 1080.0,
                ..FramePiece::default()
            }
        );
    }

    #[test]
    fn the_aperture_keeps_the_rounding_whole() {
        // the rounding belongs to the aperture: a thickness of 10 leaves a
        // requested radius of 25 a radius of 25, unshaved to match the bezel
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, true, true, true, true),
            Position::Top,
            38.0
        );

        assert_eq!(shape.inner.x, 10.0);
        assert_eq!(shape.inner.top_left, 25.0);
        assert_eq!(shape.inner.top_right, 25.0);
        assert_eq!(shape.inner.bottom_left, 25.0);
        assert_eq!(shape.inner.bottom_right, 25.0);
    }

    #[test]
    fn the_aperture_sits_below_the_docked_bar() {
        // the bar bands the hole even when no strip is drawn along its edge
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, false, false, false, false),
            Position::Top,
            38.0
        );

        assert_eq!(shape.inner.y, 38.0);
        assert_eq!(shape.inner.h, 1080.0 - 38.0);
    }

    #[test]
    fn the_aperture_sits_above_a_bottom_docked_bar() {
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, true, true, true, true),
            Position::Bottom,
            38.0
        );

        assert_eq!(shape.inner.y, 10.0);
        assert_eq!(shape.inner.h, 1080.0 - 10.0 - 38.0);
    }

    #[test]
    fn a_corner_one_side_gives_way_on_stays_square() {
        // strict edges: a disabled edge leaves the aperture flush with the
        // screen there, and the corner reads square
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, true, false, false, false),
            Position::Top,
            0.0
        );

        assert_eq!(shape.inner.top_left, 0.0);
        assert_eq!(shape.inner.bottom_left, 0.0);
        assert_eq!(shape.inner.top_right, 0.0);
        assert_eq!(shape.inner.bottom_right, 0.0);
    }

    #[test]
    fn a_corner_two_drawn_sides_meet_at_stays_rounded() {
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, false, true, true, false),
            Position::Top,
            0.0
        );

        assert_eq!(shape.inner.top_left, 0.0);
        assert_eq!(shape.inner.bottom_left, 25.0);
        assert_eq!(shape.inner.top_right, 0.0);
        assert_eq!(shape.inner.bottom_right, 0.0);
    }

    #[test]
    fn a_docked_bar_joins_the_ring_rounded() {
        // the bar band counts as a drawn side where it fills an edge: the
        // joint where a side strip grows out of the strip reads rounded
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, false, false, true, false),
            Position::Top,
            38.0
        );

        assert_eq!(shape.inner.top_left, 25.0);
        assert_eq!(shape.inner.bottom_left, 0.0);
        assert_eq!(shape.inner.top_right, 0.0);
        assert_eq!(shape.inner.bottom_right, 0.0);
    }

    #[test]
    fn a_radius_beyond_the_holes_halves_is_clamped() {
        let shape = frame_pieces(
            100.0,
            100.0,
            &frame(10.0, 80.0, true, true, true, true),
            Position::Top,
            0.0
        );

        assert_eq!(shape.inner.top_left, 40.0);
        assert_eq!(shape.inner.bottom_right, 40.0);
    }

    #[test]
    fn widths_consuming_the_screen_leave_no_hole() {
        let consuming = FrameConfig {
            enabled:   true,
            thickness: 0.0,
            rounding:  25.0,
            edges:     FrameEdges {
                top:    FrameEdge {
                    enabled: true,
                    width:   Some(400.0),
                    entries: Vec::new()
                },
                bottom: FrameEdge {
                    enabled: true,
                    width:   Some(400.0),
                    entries: Vec::new()
                },
                left:   FrameEdge {
                    enabled: true,
                    width:   Some(400.0),
                    entries: Vec::new()
                },
                right:  FrameEdge {
                    enabled: true,
                    width:   Some(400.0),
                    entries: Vec::new()
                }
            }
        };

        let shape = frame_pieces(700.0, 700.0, &consuming, Position::Top, 0.0);

        assert_eq!(shape.inner.w, 0.0);
        assert_eq!(shape.inner.h, 0.0);
        assert_eq!(shape.inner.top_left, 0.0);
    }

    #[test]
    fn no_dimension_turns_negative_or_fractional_nonsense() {
        let shape = frame_pieces(
            0.0,
            0.0,
            &frame(10.0, 25.0, true, true, true, true),
            Position::Top,
            38.0
        );

        assert_eq!(shape.outer.w, 0.0);
        assert_eq!(shape.outer.h, 0.0);
        assert_eq!(shape.inner.w, 0.0);
        assert_eq!(shape.inner.h, 0.0);
        assert_eq!(shape.inner.top_left, 0.0);
    }

    #[test]
    fn a_disabled_frame_still_answers_a_well_formed_shape() {
        let off = FrameConfig {
            enabled: false,
            ..FrameConfig::default()
        };
        let shape = frame_pieces(1920.0, 1080.0, &off, Position::Top, 38.0);

        // drawing gates on enabled elsewhere; the geometry stays a pure
        // function of the numbers it was handed
        assert_eq!(shape.outer.w, 1920.0);
        assert_eq!(shape.inner.x, 10.0);
    }
}
