//! What one reservation panel of the frame asks the compositor for.

use iced::{Anchor, KeyboardInteractivity, LayerShellSettings, OutputId};

use super::{super::MAIN_NAMESPACE, surface_layer};
use crate::config::{BarLayer, Position};

/// One of the four panels of the frame: the strips ringing the screen, one
/// per edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelEdge {
    /// The strip along the top edge of the screen.
    Top,
    /// The strip along the right edge of the screen.
    Right,
    /// The strip along the bottom edge of the screen.
    Bottom,
    /// The strip along the left edge of the screen.
    Left
}

/// Anchor band and pixel size a panel of `edge` asks for at `width`.
///
/// A zero width is anchored to all four edges instead of its own: the layer
/// shell protocol forbids a zero size along a dimension the surface is not
/// anchored to both ends of, and a compositor answers the violation by
/// killing the whole connection — every surface, not the offending one.
#[must_use]
pub fn panel_anchor(edge: PanelEdge, width: u32) -> (Anchor, (u32, u32)) {
    match (edge, width == 0) {
        (_, true) => (Anchor::all(), (0, 0)),
        (PanelEdge::Left, false) => (Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM, (width, 0)),
        (PanelEdge::Right, false) => (Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM, (width, 0)),
        (PanelEdge::Top, false) => (Anchor::TOP | Anchor::LEFT | Anchor::RIGHT, (0, width)),
        (PanelEdge::Bottom, false) => (Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT, (0, width))
    }
}

/// The margin keeping a panel clear of the band the docked bar fills.
///
/// A side panel next to a docked bar starts below the band, so the bar is
/// never pushed a second time by a transparent sliver of the same paint.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the bar height rounds within one pixel of its real band"
)]
pub const fn panel_clearance(
    dock: Position,
    edge: PanelEdge,
    bar_height: f32
) -> (i32, i32, i32, i32) {
    match (dock, edge) {
        (Position::Top, PanelEdge::Left | PanelEdge::Right) => {
            (bar_height.round() as i32, 0, 0, 0)
        }
        (Position::Bottom, PanelEdge::Left | PanelEdge::Right) => {
            (0, 0, bar_height.round() as i32, 0)
        }
        _ => (0, 0, 0, 0)
    }
}

/// Settings of the surface one panel of the frame is drawn on.
///
/// A panel is the bar's own sibling strip, not an overlay: it is anchored to
/// its edge, and while the user enabled that edge it reserves its width as
/// its exclusive zone — the desktop is pushed off the strip exactly the way
/// the bar pushes it off the top. A panel of an edge that reserves nothing
/// is not created at all: no surface, no zone, nothing the desktop has to
/// stand clear of until a config flip asks for the strip and the sync
/// builds it.
///
/// The surface carries the bar's own namespace on purpose: it is part of the
/// bar, painted in the bar palette, and the blur rule the bar asks for keys
/// on the namespace — sharing it is what blurs behind the strips without a
/// second rule or a second paint. The margin keeps the panel clear of the
/// bar band when the bar is docked to a horizontal edge the panel meets.
///
/// A zero width is anchored to all four edges instead of its own: the layer
/// shell protocol forbids a zero size along a dimension the surface is not
/// anchored to both ends of, and a compositor answers the violation by
/// killing the whole connection — every surface, not the offending one.
#[must_use]
pub fn panel_settings(
    edge: PanelEdge,
    output: Option<OutputId>,
    layer: BarLayer,
    dock: Position,
    bar_height: f32,
    width: u32
) -> LayerShellSettings {
    let (anchor, size) = panel_anchor(edge, width);
    let clearance = panel_clearance(dock, edge, bar_height);

    LayerShellSettings {
        namespace: MAIN_NAMESPACE.to_string(),
        size: Some(size),
        layer: surface_layer(layer),
        keyboard_interactivity: KeyboardInteractivity::None,
        output,
        anchor,
        exclusive_zone: i32::try_from(width).unwrap_or(i32::MAX),
        margin: clearance,
        input_region: Some(Vec::new())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_reservation_anchors_its_edge_and_reserves_its_width() {
        // a left panel is anchored between the screen's own top and bottom
        // and reserves its width along the left
        let settings = panel_settings(
            PanelEdge::Left,
            None,
            BarLayer::Top,
            Position::Bottom,
            38.0,
            10
        );

        assert_eq!(settings.anchor, Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM);
        assert_eq!(settings.size, Some((10, 0)));
        assert_eq!(settings.exclusive_zone, 10);
        assert_eq!(settings.input_region, Some(Vec::new()));
        // the margin order is (top, right, bottom, left)
        let (_top, _right, bottom, _left) = settings.margin;

        assert_eq!(bottom, 38);
    }

    #[test]
    fn a_side_panel_beside_a_top_bar_starts_below_its_band() {
        let settings = panel_settings(
            PanelEdge::Right,
            None,
            BarLayer::Top,
            Position::Top,
            30.0,
            10
        );
        // the margin order is (top, right, bottom, left)
        let (top, _right, _bottom, _left) = settings.margin;

        assert_eq!(top, 30);
    }

    #[test]
    fn a_zero_width_reservation_stays_protocol_safe() {
        // a surface sized to nothing along its own edge would be protocol
        // -invalid: a zero width anchors both ends instead
        let settings = panel_settings(PanelEdge::Top, None, BarLayer::Top, Position::Top, 30.0, 0);

        assert_eq!(settings.anchor, Anchor::all());
        assert_eq!(settings.size, Some((0, 0)));
        assert_eq!(settings.exclusive_zone, 0);
    }

    #[test]
    fn the_anchor_and_the_size_follow_the_edge_named() {
        let (anchor, size) = panel_anchor(PanelEdge::Bottom, 14);

        assert_eq!(anchor, Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        assert_eq!(size, (0, 14));

        let (anchor, size) = panel_anchor(PanelEdge::Right, 14);

        assert_eq!(anchor, Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM);
        assert_eq!(size, (14, 0));

        let (anchor, size) = panel_anchor(PanelEdge::Top, 14);

        assert_eq!(anchor, Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
        assert_eq!(size, (0, 14));
    }

    #[test]
    fn the_clearance_sits_where_the_bar_is() {
        // the margin order is (top, right, bottom, left): a top bar pushes
        // the side panels down from the top, a bottom bar holds them clear
        // from the bottom
        let below = panel_clearance(Position::Top, PanelEdge::Left, 38.0);
        let above = panel_clearance(Position::Bottom, PanelEdge::Left, 38.0);
        let none = panel_clearance(Position::Top, PanelEdge::Top, 38.0);

        assert_eq!(below, (38, 0, 0, 0));
        assert_eq!(above, (0, 0, 38, 0));
        assert_eq!(none, (0, 0, 0, 0));
    }
}
