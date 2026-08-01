//! The border and shadow the islands adopt from the compositor's windows.

/// Border of a compositor window, as the islands adopt it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowBorder {
    /// Width in the bar's layout pixels.
    pub width: f32,
    /// RGBA in unit range, the leading stop of the compositor gradient.
    pub color: [f32; 4]
}

/// Shadow of a compositor window, as the islands adopt it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowShadow {
    /// Reach of the shadow in the bar's layout pixels.
    pub range: f32,
    /// RGBA in unit range.
    pub color: [f32; 4]
}
