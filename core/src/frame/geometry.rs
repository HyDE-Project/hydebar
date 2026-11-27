//! The ring around the screen: a square screen minus a rounded aperture.
//!
//! Like the `Caelestia` border, the frame is not four glued strips but one
//! shape: the untouched screen rectangle, less an inner rectangle inset by
//! each drawn edge. The screen boundary stays square — the physical corners
//! of the output are fully painted — and every rounding lives on the inner
//! aperture, where the content faces it.
//!
//! The docked bar never sits at the screen edge by right: another panel may
//! have reserved that row first, and the compositor pushes the bar down by
//! it. The ring reads that row back from the compositor ([`super::super::
//! outputs::perch`]) and states it as `bar_offset`: the strip of screen
//! between the edge and the bar is cut out of the ring — the other panel
//! owns it — the bar's own band is cut out too, and the aperture opens at
//! the bar's far edge, where its rounding draws the smooth junction the
//! ring and the panel share.
//!
//! ```rust
//! use hydebar_core::{config::Position, frame::geometry::frame_pieces};
//! use hydebar_proto::config::FrameConfig;
//!
//! let frame = FrameConfig {
//!     enabled: true,
//!     ..FrameConfig::default()
//! };
//! let shape = frame_pieces(1920.0, 1080.0, &frame, Position::Top, 38.0, 0.0);
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

/// The outline of the frame: the outer and inner rectangles, and the cuts
/// the ring leaves clear.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameShape {
    /// The far bound of the ring: the full screen, square at the corners.
    pub outer: FramePiece,
    /// The hole carved out of it: the content area, rounded inward.
    pub inner: FramePiece,
    /// The straight bands cut out of the ring: the row of whatever panel
    /// stands before the bar, and the bar's own band it paints itself.
    pub cuts:  Vec<FramePiece>
}

/// The frame's outline for a screen.
///
/// `bar_height` is the band the docked bar reserves and `bar_offset` the
/// row of screen the compositor put between the docked edge and that band
/// — the space some other panel reserved first. The content hole sits
/// beyond the bar at `bar_offset + bar_height` even when no strip is drawn
/// there, and everything from the edge to the bar is cut out of the ring:
/// that row belongs to the panel standing on it. Each drawn edge insets
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
    bar_height: f32,
    bar_offset: f32
) -> FrameShape {
    let radius = frame
        .rounding
        .unwrap_or(FrameConfig::DEFAULT_ROUNDING)
        .max(0.0);
    let inset = |edge: &FrameEdge| {
        edge.enabled
            .then_some(edge.width.unwrap_or(frame.thickness))
            .filter(|width| *width > 0.0)
    };
    let band = matches!(dock, Position::Top | Position::Bottom);
    let band_height = if band { bar_height.max(0.0) } else { 0.0 };
    let band_offset = if band { bar_offset.max(0.0) } else { 0.0 };
    let band_reach = (band_offset + band_height).min(if band { screen_h } else { 0.0 });

    let mut top = inset(&frame.edges.top).unwrap_or(0.0);
    let mut bottom = inset(&frame.edges.bottom).unwrap_or(0.0);

    if matches!(dock, Position::Top) {
        top = top.max(band_reach);
    }

    if matches!(dock, Position::Bottom) {
        bottom = bottom.max(band_reach);
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

    let mut cuts = Vec::new();

    if band_offset > 0.0 {
        let clamped = band_offset.min(screen_h);

        cuts.push(if matches!(dock, Position::Top) {
            FramePiece {
                x: 0.0,
                y: 0.0,
                w: screen_w,
                h: clamped,
                ..FramePiece::default()
            }
        } else {
            FramePiece {
                x: 0.0,
                y: screen_h - clamped,
                w: screen_w,
                h: clamped,
                ..FramePiece::default()
            }
        });
    }

    if band_height > 0.0 {
        let bar_y = match dock {
            Position::Top => band_offset,
            Position::Bottom => (screen_h - band_offset - band_height).max(0.0)
        };

        cuts.push(FramePiece {
            x: left,
            y: bar_y,
            w: hole_w,
            h: band_height.min(screen_h),
            ..FramePiece::default()
        });
    }

    FrameShape {
        outer,
        inner,
        cuts
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

/// Whether the frame paints the point `(x, y)`.
///
/// The painted set is the outer screen rect, less the rounded inner hole,
/// less every cut — the very set the canvas program fills with its even-odd
/// path, so the geometry tests can state, pixel by pixel, what a scene
/// looks like without opening a window: a physical corner is painted, the
/// middle of the aperture is not, the bar's band and the row before it are
/// never painted, and the ring carries no second coat anywhere.
///
/// ```rust
/// use hydebar_core::{
///     config::Position,
///     frame::geometry::{covers, frame_pieces}
/// };
/// use hydebar_proto::config::FrameConfig;
///
/// let frame = FrameConfig {
///     enabled: true,
///     ..FrameConfig::default()
/// };
/// let shape = frame_pieces(64.0, 64.0, &frame, Position::Top, 0.0, 0.0);
///
/// assert!(covers(&shape, 0.0, 0.0));
/// assert!(!covers(&shape, 32.0, 32.0));
/// ```
#[must_use]
pub fn covers(shape: &FrameShape, x: f32, y: f32) -> bool {
    inside(shape.outer, x, y)
        && !inside(shape.inner, x, y)
        && shape.cuts.iter().all(|cut| !inside(*cut, x, y))
}

/// Whether the point `(x, y)` falls inside one rounded rectangle.
fn inside(piece: FramePiece, x: f32, y: f32) -> bool {
    let FramePiece {
        x: left,
        y: top,
        w,
        h,
        top_left,
        top_right,
        bottom_right,
        bottom_left
    } = piece;

    if x < left || x > left + w || y < top || y > top + h {
        return false;
    }

    let horizontal = (
        left + top_left.max(bottom_left),
        left + w - top_right.max(bottom_right)
    );
    let vertical = (
        top + top_left.max(top_right),
        top + h - bottom_left.max(bottom_right)
    );

    if x >= horizontal.0 && x <= horizontal.1 {
        return true;
    }

    if y >= vertical.0 && y <= vertical.1 {
        return true;
    }

    let corner = if x < horizontal.0 && y < vertical.0 {
        Some(((left + top_left, top + top_left), top_left))
    } else if x > horizontal.1 && y < vertical.0 {
        Some(((left + w - top_right, top + top_right), top_right))
    } else if x > horizontal.1 && y > vertical.1 {
        Some((
            (left + w - bottom_right, top + h - bottom_right),
            bottom_right
        ))
    } else if x < horizontal.0 && y > vertical.1 {
        Some(((left + bottom_left, top + h - bottom_left), bottom_left))
    } else {
        None
    };

    corner
        .is_some_and(|((center_x, center_y), radius)| (x - center_x).hypot(y - center_y) <= radius)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use hydebar_proto::config::{FrameEdge, FrameEdges};

    use super::*;

    /// Builds a frame config from enabled flags, in the edge order
    /// `[top, bottom, left, right]`.
    fn frame(thickness: f32, rounding: f32, enabled: [bool; 4]) -> FrameConfig {
        let [top, bottom, left, right] = enabled;
        let edge = |enabled: bool| FrameEdge {
            enabled,
            width: None,
            entries: Vec::new()
        };

        FrameConfig {
            enabled: true,
            thickness,
            rounding: Some(rounding),
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
            &frame(10.0, 25.0, [true, true, true, true]),
            Position::Top,
            38.0,
            0.0
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
            &frame(10.0, 25.0, [true, true, true, true]),
            Position::Top,
            38.0,
            0.0
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
            &frame(10.0, 25.0, [false, false, false, false]),
            Position::Top,
            38.0,
            0.0
        );

        assert_eq!(shape.inner.y, 38.0);
        assert_eq!(shape.inner.h, 1080.0 - 38.0);
    }

    #[test]
    fn the_aperture_sits_above_a_bottom_docked_bar() {
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, [true, true, true, true]),
            Position::Bottom,
            38.0,
            0.0
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
            &frame(10.0, 25.0, [true, false, false, false]),
            Position::Top,
            0.0,
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
            &frame(10.0, 25.0, [false, true, true, false]),
            Position::Top,
            0.0,
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
            &frame(10.0, 25.0, [false, false, true, false]),
            Position::Top,
            38.0,
            0.0
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
            &frame(10.0, 80.0, [true, true, true, true]),
            Position::Top,
            0.0,
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
            rounding:  Some(25.0),
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

        let shape = frame_pieces(700.0, 700.0, &consuming, Position::Top, 0.0, 0.0);

        assert_eq!(shape.inner.w, 0.0);
        assert_eq!(shape.inner.h, 0.0);
        assert_eq!(shape.inner.top_left, 0.0);
    }

    #[test]
    fn no_dimension_turns_negative_or_fractional_nonsense() {
        let shape = frame_pieces(
            0.0,
            0.0,
            &frame(10.0, 25.0, [true, true, true, true]),
            Position::Top,
            38.0,
            0.0
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
        let shape = frame_pieces(1920.0, 1080.0, &off, Position::Top, 38.0, 0.0);

        // drawing gates on enabled elsewhere; the geometry stays a pure
        // function of the numbers it was handed
        assert_eq!(shape.outer.w, 1920.0);
        assert_eq!(shape.inner.x, 10.0);
    }

    /// Builds a frame config from per-edge widths, in the edge order
    /// `[top, bottom, left, right]`; a `None` keeps the shared thickness.
    fn frame_widths(thickness: f32, rounding: f32, widths: [Option<f32>; 4]) -> FrameConfig {
        let [top, bottom, left, right] = widths;
        let edge = |width: Option<f32>| FrameEdge {
            enabled: true,
            width,
            entries: Vec::new()
        };

        FrameConfig {
            enabled: true,
            thickness,
            rounding: Some(rounding),
            edges: FrameEdges {
                top:    edge(top),
                bottom: edge(bottom),
                left:   edge(left),
                right:  edge(right)
            }
        }
    }

    #[test]
    fn a_small_square_scene_paints_corners_and_spares_the_middle() {
        let shape = frame_pieces(
            64.0,
            64.0,
            &frame(8.0, 16.0, [true, true, true, true]),
            Position::Top,
            0.0,
            0.0
        );

        // every physical corner is painted: the screen boundary is square
        assert!(covers(&shape, 0.0, 0.0));
        assert!(covers(&shape, 63.0, 0.0));
        assert!(covers(&shape, 0.0, 63.0));
        assert!(covers(&shape, 63.0, 63.0));

        // the middle of each strip is painted exactly once
        assert!(covers(&shape, 32.0, 4.0));
        assert!(covers(&shape, 32.0, 60.0));
        assert!(covers(&shape, 4.0, 32.0));
        assert!(covers(&shape, 60.0, 32.0));

        // the aperture stays clear, middle and rounded corner alike
        assert!(!covers(&shape, 32.0, 32.0));
        assert!(!covers(&shape, 16.0, 16.0));
        assert!(!covers(&shape, 46.0, 46.0));

        // the ring rounds around the aperture's arc, not through it
        assert!(covers(&shape, 4.0, 4.0));
        assert!(covers(&shape, 52.0, 52.0));
        assert!(covers(&shape, 56.0, 10.0));
        assert!(covers(&shape, 8.0, 54.0));

        // the straight bands beside the arcs read as the hole, not the ring
        assert!(!covers(&shape, 48.0, 16.0));
        assert!(!covers(&shape, 16.0, 48.0));
        assert!(!covers(&shape, 12.0, 32.0));
        assert!(!covers(&shape, 50.0, 32.0));
    }

    #[test]
    fn an_asymmetric_scene_keeps_every_edge_in_its_own_width() {
        let shape = frame_pieces(
            96.0,
            64.0,
            &frame_widths(10.0, 12.0, [Some(5.0), Some(13.0), Some(17.0), Some(9.0)]),
            Position::Top,
            0.0,
            0.0
        );

        assert_eq!((shape.inner.x, shape.inner.y), (17.0, 5.0));

        assert!(covers(&shape, 0.0, 0.0));
        assert!(covers(&shape, 95.0, 63.0));
        assert!(covers(&shape, 48.0, 2.0));
        assert!(covers(&shape, 48.0, 60.0));
        assert!(covers(&shape, 2.0, 32.0));
        assert!(covers(&shape, 93.0, 32.0));
        assert!(!covers(&shape, 48.0, 32.0));
    }

    #[test]
    fn a_portrait_scene_rings_upright() {
        let shape = frame_pieces(
            64.0,
            96.0,
            &frame(8.0, 16.0, [true, true, true, true]),
            Position::Top,
            0.0,
            0.0
        );

        assert!(covers(&shape, 0.0, 0.0));
        assert!(covers(&shape, 63.0, 95.0));
        assert!(covers(&shape, 32.0, 4.0));
        assert!(covers(&shape, 32.0, 92.0));
        assert!(!covers(&shape, 32.0, 48.0));
    }

    #[test]
    fn a_docked_bar_leaves_its_own_band_clear() {
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, [true, true, true, true]),
            Position::Top,
            38.0,
            0.0
        );

        // the bar paints its band itself: the ring owns no pixel of it,
        // while the physical corners beside the band belong to the ring
        assert!(!covers(&shape, 960.0, 19.0));
        assert!(covers(&shape, 0.0, 0.0));
        assert!(covers(&shape, 1919.0, 0.0));

        // the side strips run from the band down, paint below it
        assert!(covers(&shape, 5.0, 100.0));
        assert!(covers(&shape, 1915.0, 100.0));
        assert!(covers(&shape, 0.0, 100.0));

        // the free corners and the aperture middle read as ever
        assert!(covers(&shape, 0.0, 1079.0));
        assert!(covers(&shape, 1919.0, 1079.0));
        assert!(!covers(&shape, 960.0, 559.0));
    }

    #[test]
    fn a_panel_before_the_bar_keeps_its_row_clear() {
        // waybar stands on the top row, hydebar's bar sits below it: the ring
        // paints neither row, and its rounding opens at the bar's far edge
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, [false, true, true, true]),
            Position::Top,
            38.0,
            38.0
        );

        // the row of the panel before the bar and the bar's own band stay clear
        assert!(!covers(&shape, 960.0, 19.0));
        assert!(!covers(&shape, 960.0, 57.0));
        assert!(!covers(&shape, 5.0, 19.0));

        // the side strips run alongside the bar and below it
        assert!(covers(&shape, 5.0, 57.0));
        assert!(covers(&shape, 1915.0, 57.0));
        assert!(covers(&shape, 5.0, 100.0));

        // the aperture opens at the bar's far edge, rounding the junction
        assert_eq!(shape.inner.y, 76.0);
        assert_eq!(shape.inner.top_left, 25.0);
        assert!(!covers(&shape, 20.0, 90.0));
        assert!(covers(&shape, 9.0, 95.0));
        assert!(!covers(&shape, 960.0, 559.0));
    }

    #[test]
    fn the_cuts_and_the_hole_never_share_a_pixel() {
        // the even-odd fill clears each exactly once only while the cuts and
        // the hole stay disjoint: the hole's docked inset always reaches the
        // bar's far edge, wherever the bar stands
        for dock in [Position::Top, Position::Bottom] {
            for offset in [0.0, 38.0] {
                let shape = frame_pieces(
                    1920.0,
                    1080.0,
                    &frame(10.0, 25.0, [true, true, true, true]),
                    dock,
                    38.0,
                    offset
                );

                assert_eq!(shape.cuts.len(), usize::from(offset > 0.0) + 1);

                for cut in &shape.cuts {
                    assert_eq!((cut.top_left, cut.bottom_right), (0.0, 0.0));

                    let clear = (cut.x + cut.w <= shape.inner.x)
                        || (cut.y + cut.h <= shape.inner.y)
                        || (cut.x >= shape.inner.x + shape.inner.w)
                        || (cut.y >= shape.inner.y + shape.inner.h);

                    assert!(clear, "the cut {cut:?} overlaps the hole");
                }

                if dock == Position::Top {
                    assert!(shape.inner.y >= 38.0 + offset);
                } else {
                    assert!(shape.inner.y + shape.inner.h <= 1080.0 - 38.0 - offset);
                }
            }
        }

        let bare = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, [true, true, true, true]),
            Position::Top,
            0.0,
            0.0
        );

        assert_eq!(bare.cuts, []);
    }

    #[test]
    fn a_switched_off_edge_paints_nothing() {
        let shape = frame_pieces(
            1920.0,
            1080.0,
            &frame(10.0, 25.0, [true, true, false, true]),
            Position::Top,
            0.0,
            0.0
        );

        assert!(!covers(&shape, 2.0, 540.0));
        assert!(covers(&shape, 1918.0, 540.0));
        assert!(!covers(&shape, 960.0, 540.0));
    }

    #[test]
    fn no_number_outside_the_numbers_paints() {
        let shape = frame_pieces(
            64.0,
            64.0,
            &frame(8.0, 16.0, [true, true, true, true]),
            Position::Top,
            0.0,
            0.0
        );

        // a NaN coordinate answers no corner, no band and no straight run:
        // the point is painted nowhere rather than anywhere
        assert!(!covers(&shape, f32::NAN, 32.0));
        assert!(!covers(&shape, 32.0, f32::NAN));
        assert!(!covers(&shape, f32::NAN, f32::NAN));
    }

    #[test]
    fn a_consumed_screen_paints_its_ring_and_nothing_else() {
        let consuming = FrameConfig {
            enabled:   true,
            thickness: 0.0,
            rounding:  Some(25.0),
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
        let shape = frame_pieces(700.0, 700.0, &consuming, Position::Top, 0.0, 0.0);

        assert!(covers(&shape, 0.0, 0.0));
        assert!(covers(&shape, 699.0, 699.0));
        assert!(!covers(&shape, 400.0, 400.0));
    }
}
