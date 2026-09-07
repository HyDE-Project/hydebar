//! Frame slice of the appearance configuration: the ring around the screen.
//!
//! The frame is an opt-in perimeter the bar draws on its desk surface, after
//! the fashion of the `Caelestia` shell: a strip along any combination of
//! screen edges, with rounded outer corners and a rounded joint where a
//! strip meets the bar band. Each edge is switched and sized on its own, so
//! a bar docked to the top can stand on a left edge alone, on the left and
//! bottom edges, or on the full ring.
//!
//! ```toml
//! [appearance.frame]
//! enabled = true
//!
//! [appearance.frame.edges]
//! top = false
//!
//! [appearance.frame.widths]
//! left = 14.0
//! ```

use serde::Deserialize;

/// Default thickness of a frame strip, in pixels.
const fn default_thickness() -> f32 {
    10.0
}

/// Default radius of the frame's outer corners and bar joints, in pixels.
const fn default_rounding() -> f32 {
    25.0
}

/// A frame edge is drawn unless the user switches it off.
const fn enabled_edge() -> bool {
    true
}

/// Which screen edges carry the frame.
///
/// Every edge defaults to drawn. The edge the bar is docked to never paints
/// its straight run: the bar band covers it, and a translucent bar would pick
/// up a second layer of colour from a strip hiding underneath.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each edge is an independent configuration switch"
)]
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameEdges {
    /// Whether the top edge of the screen carries the frame.
    #[serde(default = "enabled_edge")]
    pub top:    bool,
    /// Whether the bottom edge of the screen carries the frame.
    #[serde(default = "enabled_edge")]
    pub bottom: bool,
    /// Whether the left edge of the screen carries the frame.
    #[serde(default = "enabled_edge")]
    pub left:   bool,
    /// Whether the right edge of the screen carries the frame.
    #[serde(default = "enabled_edge")]
    pub right:  bool
}

impl Default for FrameEdges {
    fn default() -> Self {
        Self {
            top:    true,
            bottom: true,
            left:   true,
            right:  true
        }
    }
}

/// Per-edge widths of the frame, overriding the shared thickness.
///
/// Every width defaults to unset, which paints the edge at `thickness`.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameWidths {
    /// Width of the top strip, when it is not the shared thickness.
    #[serde(default)]
    pub top:    Option<f32>,
    /// Width of the bottom strip, when it is not the shared thickness.
    #[serde(default)]
    pub bottom: Option<f32>,
    /// Width of the left strip, when it is not the shared thickness.
    #[serde(default)]
    pub left:   Option<f32>,
    /// Width of the right strip, when it is not the shared thickness.
    #[serde(default)]
    pub right:  Option<f32>
}

/// The ring the bar draws around the screen.
///
/// Disabled by default: the frame paints on the desk surface of every
/// output, and a user who never asked for one should see none of it.
///
/// The frame states no colour of its own: it paints the bar palette's
/// background at the island opacity, the exact expression the island
/// buttons rest at, so the ring and the bar are one paint by construction
/// rather than by matching numbers.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct FrameConfig {
    /// Whether the frame is painted on the desk surface.
    #[serde(default)]
    pub enabled:   bool,
    /// Thickness of a frame strip, in pixels.
    #[serde(default = "default_thickness")]
    pub thickness: f32,
    /// Radius of the outer corners and of the joints with the bar band.
    #[serde(default = "default_rounding")]
    pub rounding:  f32,
    /// Which screen edges carry the frame.
    #[serde(default)]
    pub edges:     FrameEdges,
    /// Per-edge widths, each falling back to `thickness` when unset.
    #[serde(default)]
    pub widths:    FrameWidths
}

impl Default for FrameConfig {
    fn default() -> Self {
        Self {
            enabled:   false,
            thickness: default_thickness(),
            rounding:  default_rounding(),
            edges:     FrameEdges::default(),
            widths:    FrameWidths::default()
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    fn configured() -> FrameConfig {
        toml::from_str(
            r"
            enabled = true
            thickness = 6.0
            rounding = 12.0

            [edges]
            top = false
            "
        )
        .expect("the frame fixture names a valid configuration")
    }

    #[test]
    fn unset_edges_stay_drawn() {
        let edges = FrameEdges::default();

        assert!(edges.top && edges.bottom && edges.left && edges.right);
    }

    #[test]
    fn a_missing_frame_section_stays_off() {
        let config: FrameConfig = toml::from_str("").expect("empty tables parse");

        assert!(!config.enabled);
        assert_eq!(config.thickness, default_thickness());
        assert_eq!(config.rounding, default_rounding());
        assert_eq!(config.edges, FrameEdges::default());
    }

    #[test]
    fn an_edge_switches_off_on_its_own() {
        let config = configured();

        assert!(config.enabled);
        assert!(!config.edges.top);
        assert!(config.edges.bottom && config.edges.left && config.edges.right);
    }

    #[test]
    fn an_edge_width_overrides_the_shared_thickness() {
        let config: FrameConfig = toml::from_str(
            r"
            [widths]
            left = 14.0
            "
        )
        .expect("the widths fixture names a valid configuration");

        assert_eq!(config.widths.left, Some(14.0));
        assert_eq!(config.widths.top, None);
    }
}
