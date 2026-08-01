//! Where the bar sits on the output: the edge it hugs and the compositor
//! layer it is placed on.

use serde::Deserialize;

/// Bar placement configuration.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    /// Render the bar at the top of the output.
    #[default]
    Top,
    /// Render the bar at the bottom of the output.
    Bottom
}

/// Compositor layer the bar surface is placed on.
///
/// Compositors composite the blur source from the background and bottom
/// levels, so a bar that should be blurred behind has to sit on [`Top`] or
/// above.
///
/// [`Top`]: BarLayer::Top
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarLayer {
    /// Behind every window, together with the wallpaper.
    Background,
    /// Behind windows but above the wallpaper.
    #[default]
    Bottom,
    /// Above windows, alongside most status bars.
    Top,
    /// Above everything, including fullscreen surfaces.
    Overlay
}
