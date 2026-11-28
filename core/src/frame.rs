//! The ring around the screen, drawn on a surface of its own.
//!
//! The ring is one shape: the screen, square at its corners, less an inner
//! content aperture rounded where the content faces it — the way the
//! `Caelestia` shell frames its desktop. [`geometry`] turns the
//! configuration into that outline, the canvas program in [`program`] fills
//! a single even-odd path of it, and [`panel_widths`] names the widths of
//! the layer-shell reservations that push the desktop off the drawn edges;
//! drawing and reservation are separate concerns all the way down. The row
//! of screen a docked bar stands on is read back from the compositor, so
//! the ring opens at the bar's far edge wherever a panel before it pushed
//! the bar down.
//!
//! ```rust
//! use hydebar_core::{config::Position, frame::geometry::frame_pieces};
//! use hydebar_proto::config::{FrameConfig, FrameEdge, FrameEdges};
//!
//! let frame = FrameConfig {
//!     enabled: true,
//!     edges: FrameEdges {
//!         top: FrameEdge {
//!             enabled: false,
//!             ..FrameEdge::default()
//!         },
//!         ..FrameEdges::default()
//!     },
//!     ..FrameConfig::default()
//! };
//! let shape = frame_pieces(1920.0, 1080.0, &frame, Position::Top, 38.0, 0.0);
//!
//! assert_eq!(shape.inner.x, 10.0);
//! assert!(shape.inner.w > 0.0 && shape.inner.h > 0.0);
//! ```

use hydebar_proto::config::{FrameConfig, FrameEdge, Position};

pub mod geometry;
pub mod program;
pub mod shm;

/// Widths of the four panel reservations of one screen, scaled the way the
/// bar's own height scales.
///
/// A reservation exists only for a drawn edge of an enabled frame: the
/// switched-off strip reserves nothing, and the edge the bar is docked to is
/// the bar's own exclusive zone, never reserved a second time. Each drawn
/// edge reserves its own override width when one is set, the shared
/// thickness else, and the scale factor stretches the pixels the way it
/// stretches the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PanelWidths {
    /// Width of the top strip, in physical pixels.
    pub top:    u32,
    /// Width of the right strip, in physical pixels.
    pub right:  u32,
    /// Width of the bottom strip, in physical pixels.
    pub bottom: u32,
    /// Width of the left strip, in physical pixels.
    pub left:   u32
}

/// Widths of the four panel reservations, scaled by the output's scale
/// factor.
///
/// The docked bar already reserves its own edge, so that edge receives no
/// panel at all. An edge the user switched off reserves nothing as well: the
/// surface exists so a config flip only has to resize it, and a zero width
/// is what a switched-off strip looks like. A drawn edge reserves its own
/// override width when one is set, the shared thickness else, and the scale
/// factor stretches the pixels the way it stretches the bar.
#[must_use]
pub fn panel_widths(frame: &FrameConfig, dock: Position, scale_factor: f64) -> PanelWidths {
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "strip widths are small positive pixel counts"
    )]
    let scaled = |pixels: f32, scale: f64| -> u32 { (pixels * scale as f32) as u32 };

    let enabled = frame.enabled;

    let width = |edge: &FrameEdge| {
        if enabled && edge.enabled {
            edge.width.unwrap_or(frame.thickness)
        } else {
            0.0
        }
    };

    let docked = match dock {
        Position::Top => |panel: &mut PanelWidths| panel.top = 0,
        Position::Bottom => |panel: &mut PanelWidths| panel.bottom = 0,
        Position::Left => |panel: &mut PanelWidths| panel.left = 0,
        Position::Right => |panel: &mut PanelWidths| panel.right = 0
    };

    let mut widths = PanelWidths {
        top:    scaled(width(&frame.edges.top), scale_factor),
        right:  scaled(width(&frame.edges.right), scale_factor),
        bottom: scaled(width(&frame.edges.bottom), scale_factor),
        left:   scaled(width(&frame.edges.left), scale_factor)
    };

    docked(&mut widths);

    widths
}
