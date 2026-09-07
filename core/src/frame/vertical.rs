//! Vertical strips of the frame.
//!
//! A vertical strip runs between the horizontal strips — or between the bar
//! band and the far edge when the bar is docked to that side. [`vertical_end`]
//! reports where one starts and stops; [`end_radii`] how round each end is.
//!
//! ```rust
//! use hydebar_core::frame::vertical::{end_radii, vertical_end};
//! use hydebar_proto::config::Position;
//!
//! assert_eq!(end_radii(true, false, 25.0), (25.0, 25.0));
//!
//! let (top, bottom) = vertical_end(1080.0, 10.0, Position::Top, 38.0, false, true);
//!
//! assert_eq!((top, bottom), (38.0, 1070.0));
//! ```

use hydebar_proto::config::Position;

/// Where a vertical strip starts and stops on the screen.
///
/// The bar band owns the docked edge, so a strip beside the bar begins where
/// the band ends; otherwise the strip runs edge to edge, stopping below a
/// drawn horizontal strip and above one. The caller drops the strip when the
/// end does not clear the start.
#[must_use]
pub fn vertical_end(
    screen_h: f32,
    strip: f32,
    dock: Position,
    bar_height: f32,
    top_drawn: bool,
    bottom_drawn: bool
) -> (f32, f32) {
    let top = if matches!(dock, Position::Top) {
        bar_height.max(0.0)
    } else if top_drawn {
        strip
    } else {
        0.0
    };
    let bottom = if matches!(dock, Position::Bottom) {
        (screen_h - bar_height).clamp(0.0, screen_h)
    } else if bottom_drawn {
        screen_h - strip
    } else {
        screen_h
    };

    (top, bottom)
}

/// Outer and inner radii of one strip end.
///
/// A joint with the bar band rounds both corners; a screen corner rounds the
/// outer one; a meeting with another strip stays square.
#[must_use]
pub const fn end_radii(joint: bool, corner: bool, radius: f32) -> (f32, f32) {
    if joint {
        (radius, radius)
    } else if corner {
        (radius, 0.0)
    } else {
        (0.0, 0.0)
    }
}
