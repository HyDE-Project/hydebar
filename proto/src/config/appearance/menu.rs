//! Menu-specific slice of the appearance configuration.

use serde::Deserialize;

use super::defaults::{default_opacity, opacity_deserializer};

/// Menu-specific appearance configuration.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct MenuAppearance {
    #[serde(deserialize_with = "opacity_deserializer", default = "default_opacity")]
    /// Opacity of the menu surface itself.
    pub opacity:  f32,
    #[serde(default)]
    /// Opacity of the shade drawn over the screen behind a menu.
    pub backdrop: f32
}

impl Default for MenuAppearance {
    fn default() -> Self {
        Self {
            opacity:  default_opacity(),
            backdrop: f32::default()
        }
    }
}
