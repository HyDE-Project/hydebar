//! Frame slice of the appearance configuration: the ring around the screen.
//!
//! The frame is an opt-in perimeter the bar draws on a surface of its own,
//! after the fashion of the `Caelestia` shell: strips along any combination
//! of screen edges, with rounded outer corners and a rounded joint where a
//! strip meets the bar band. Each edge is switched, sized and furnished with
//! widgets on its own, so the ring can stand on the left edge alone, on the
//! left and bottom edges, or around the full screen.
//!
//! ```toml
//! [appearance.frame]
//! enabled = true
//! thickness = 10.0
//! rounding = 25.0
//!
//! [appearance.frame.edges.left]
//! enabled = true
//! width = 38.0
//! entries = [
//!     { id = "clock", alignment = "center" }
//! ]
//!
//! [appearance.frame.edges.top]
//! enabled = false
//! ```

use serde::Deserialize;

/// Default thickness of a frame strip, in pixels.
const fn default_thickness() -> f32 {
    10.0
}

/// Default radius of the frame's outer corners and bar joints, in pixels.
pub(super) const fn default_rounding() -> f32 {
    25.0
}

/// A frame edge is drawn unless the user switches it off.
const fn enabled_edge() -> bool {
    true
}

/// Where an entry sits along its strip.
///
/// The strip runs from one end of its edge to the other; an entry pinned to
/// `start` hugs the first end, an entry pinned to `end` hugs the last, and a
/// `center` entry keeps to the middle.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntryAlignment {
    /// Pinned to the first end of the strip.
    Start,
    /// Kept to the middle of the strip.
    #[default]
    Center,
    /// Pinned to the last end of the strip.
    End
}

/// One widget the user placed on a strip.
///
/// The placement is the entry's own business: an entry names the widget by
/// its module identifier and states where along the strip the widget stands,
/// so the same widget can sit on any edge without the widget itself knowing
/// what an edge is.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FrameEntry {
    /// Identifier of the widget module placed here.
    pub id:        String,
    /// Where along the strip the widget stands.
    #[serde(default)]
    pub alignment: EntryAlignment
}

/// One edge of the screen as the user configured it.
///
/// An edge carries its own switch, its own width override and its own list
/// of widget entries. Every knob defaults sensibly: the edge is drawn, it is
/// as wide as the shared `thickness`, and it carries no widgets.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct FrameEdge {
    /// Whether the strip along this edge is drawn at all.
    #[serde(default = "enabled_edge")]
    pub enabled: bool,
    /// Width of the strip, when it is not the shared `thickness`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width:   Option<f32>,
    /// Widgets the user placed on this strip, in paint order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<FrameEntry>
}

impl Default for FrameEdge {
    /// An edge with no table of its own is drawn: the frame rings the whole
    /// screen unless the user names an edge and switches it off.
    fn default() -> Self {
        Self {
            enabled: true,
            width:   None,
            entries: Vec::new()
        }
    }
}

impl FrameEdge {
    /// An edge drawn at the shared thickness with no widgets of its own.
    ///
    /// The constructor is const so fixtures and code can name a drawn edge
    /// without touching the deserializer.
    #[must_use]
    pub const fn drawn() -> Self {
        Self {
            enabled: true,
            width:   None,
            entries: Vec::new()
        }
    }
}

/// Which screen edges carry the frame.
///
/// Every edge defaults to drawn. The edge the bar is docked to never paints
/// its straight run: the bar band covers it, and a translucent bar would pick
/// up a second layer of colour from a strip hiding underneath.
#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
pub struct FrameEdges {
    /// How the top edge of the screen is furnished.
    #[serde(default = "enabled_top")]
    pub top:    FrameEdge,
    /// How the bottom edge of the screen is furnished.
    #[serde(default)]
    pub bottom: FrameEdge,
    /// How the left edge of the screen is furnished.
    #[serde(default)]
    pub left:   FrameEdge,
    /// How the right edge of the screen is furnished.
    #[serde(default)]
    pub right:  FrameEdge
}

/// The top edge is drawn by default, like every other edge.
const fn enabled_top() -> FrameEdge {
    FrameEdge {
        enabled: true,
        width:   None,
        entries: Vec::new()
    }
}

/// The ring the bar draws around the screen.
///
/// Disabled by default: the frame paints on a surface of its own on every
/// output, and a user who never asked for one should see none of it.
///
/// The frame states no colour of its own: it paints the bar palette's
/// background at the island opacity, the exact expression the island
/// buttons rest at, so the ring and the bar are one paint by construction
/// rather than by matching numbers.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct FrameConfig {
    /// Whether the frame is painted around the screen.
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

            [edges.top]
            enabled = false
            "
        )
        .expect("the frame fixture names a valid configuration")
    }

    #[test]
    fn unset_edges_stay_drawn() {
        let edges = FrameEdges::default();

        assert!(edges.top.enabled);
        assert!(edges.bottom.enabled);
        assert!(edges.left.enabled);
        assert!(edges.right.enabled);
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
        assert!(!config.edges.top.enabled);
        assert!(config.edges.bottom.enabled);
        assert!(config.edges.left.enabled);
        assert!(config.edges.right.enabled);
    }

    #[test]
    fn an_edge_width_overrides_the_shared_thickness() {
        let config: FrameConfig = toml::from_str(
            r"
            [edges.left]
            width = 14.0
            "
        )
        .expect("the widths fixture names a valid configuration");

        assert_eq!(config.edges.left.width, Some(14.0));
        assert_eq!(config.edges.top.width, None);
    }

    #[test]
    fn an_entry_names_its_widget_and_its_place() {
        let config: FrameConfig = toml::from_str(
            r"
            [edges.left]
            enabled = true
            width = 38.0
            entries = [
                { id = 'clock', alignment = 'center' },
                { id = 'workspaces', alignment = 'start' }
            ]
            "
        )
        .expect("the entries fixture names a valid configuration");

        assert_eq!(config.edges.left.entries.len(), 2);
        assert_eq!(config.edges.left.entries[0].id, "clock");
        assert_eq!(
            config.edges.left.entries[0].alignment,
            EntryAlignment::Center
        );
        assert_eq!(config.edges.left.entries[1].id, "workspaces");
        assert_eq!(
            config.edges.left.entries[1].alignment,
            EntryAlignment::Start
        );
    }

    #[test]
    fn an_entry_defaults_to_the_middle_of_its_strip() {
        let config: FrameConfig = toml::from_str(
            r"
            [edges.left]
            entries = [{ id = 'clock' }]
            "
        )
        .expect("the plain entry fixture names a valid configuration");

        assert_eq!(
            config.edges.left.entries[0].alignment,
            EntryAlignment::Center
        );
    }
}
