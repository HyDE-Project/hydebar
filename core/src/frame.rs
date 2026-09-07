//! The ring around the screen, drawn on a surface of its own.
//!
//! The frame is a set of strips along the screen edges the user switches on
//! one by one, after the fashion of the `Caelestia` shell. [`geometry`] turns
//! the configuration into disjoint painted pieces with per-corner radii — a
//! strip beside the bar stops where the bar band ends and rounds the joint —
//! so the canvas program in [`program`] only fills the pieces and never
//! reasons about edges itself.
//!
//! ```rust
//! use hydebar_core::{config::Position, frame::geometry::frame_pieces};
//! use hydebar_proto::config::{FrameConfig, FrameEdges};
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
//! let pieces = frame_pieces(1920.0, 1080.0, &frame, Position::Top, 38.0);
//!
//! assert!(pieces.iter().all(|piece| piece.w > 0.0 && piece.h > 0.0));
//! ```

use hydebar_proto::config::{FrameConfig, FrameEdge};

pub mod geometry;
pub mod program;
pub mod vertical;


/// Widths of the four panels of one screen, scaled like the bar's own height.
///
/// Every edge paints at its own override when one is set, at the shared
/// thickness else, and the scale factor stretches the pixels the way it
/// stretches the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Widths of the four panels, scaled by the output's scale factor.
///
/// An edge the user switched off gets no width at all: the panel exists so a
/// config flip only has to resize it, but a switched-off strip reserves
/// nothing. A drawn edge paints at its own override when one is set, at the
/// shared thickness else, and the scale factor stretches the pixels the way
/// it stretches the bar.
#[must_use]
pub fn panel_widths(frame: &FrameConfig, scale_factor: f64) -> PanelWidths {
    let width = |edge: &FrameEdge| {
        if edge.enabled {
            edge.width.unwrap_or(frame.thickness)
        } else {
            0.0
        }
    };

    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "strip widths are small positive pixel counts"
    )]
    let scaled = |pixels: f32| (pixels * scale_factor as f32) as u32;

    PanelWidths {
        top:    scaled(width(&frame.edges.top)),
        right:  scaled(width(&frame.edges.right)),
        bottom: scaled(width(&frame.edges.bottom)),
        left:   scaled(width(&frame.edges.left))
    }
}
