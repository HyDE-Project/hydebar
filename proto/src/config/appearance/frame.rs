//! Frame slice of the appearance configuration: the ring around the screen.
//!
//! The frame is an opt-in perimeter the bar draws on its own layer surface,
//! after the fashion of the `Caelestia` shell: a strip along any combination
//! of screen edges, with rounded outer corners and a rounded joint where a
//! strip meets the bar band. Each edge is switched on its own, so a bar
//! docked to the top can stand on a left edge alone, on the left and bottom
//! edges, or on the full ring.
//!
//! ```toml
//! [appearance.frame]
//! enabled = true
//!
//! [appearance.frame.edges]
//! top = false
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

/// The ring the bar draws around the screen.
///
/// Disabled by default: the frame is a surface of its own on every output,
/// and a user who never asked for one should not pay for it.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct FrameConfig {
    /// Whether the frame surface is created and drawn.
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
    pub edges:     FrameEdges
}

impl Default for FrameConfig {
    fn default() -> Self {
        Self {
            enabled:   false,
            thickness: default_thickness(),
            rounding:  default_rounding(),
            edges:     FrameEdges::default()
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
}
