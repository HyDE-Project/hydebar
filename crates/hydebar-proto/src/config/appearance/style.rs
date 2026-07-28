//! Rendering style the bar draws its modules with.

use serde::Deserialize;

/// Enumeration of available appearance styles.
#[derive(Deserialize, Default, Copy, Clone, Eq, PartialEq, Debug)]
pub enum AppearanceStyle {
    /// Render modules with island-style backgrounds.
    #[default]
    Islands,
    /// Render modules with a flat solid background.
    Solid,
    /// Render modules with gradients.
    Gradient
}
