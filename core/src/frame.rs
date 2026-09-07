//! The ring around the screen, drawn on a surface of its own.
//!
//! The frame is a set of strips along the screen edges the user switches on
//! one by one, after the fashion of the `Caelestia` shell. [`geometry`] turns
//! the configuration into disjoint painted pieces with per-corner radii, so
//! the canvas program in [`program`] only fills rounded rectangles and never
//! reasons about edges itself.
//!
//! ```rust
//! use hydebar_core::{config::Position, frame::geometry::frame_pieces};
//! use hydebar_proto::config::{FrameConfig, FrameEdges};
//!
//! let frame = FrameConfig {
//!     enabled: true,
//!     edges: FrameEdges {
//!         top: false,
//!         ..FrameEdges::default()
//!     },
//!     ..FrameConfig::default()
//! };
//! let pieces = frame_pieces(1920.0, 1080.0, &frame, Position::Top, 38.0);
//!
//! assert!(pieces.iter().all(|piece| piece.w > 0.0 && piece.h > 0.0));
//! ```

pub mod geometry;
pub mod program;
pub mod vertical;
