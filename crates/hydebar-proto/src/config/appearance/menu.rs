//! Menu-specific slice of the appearance configuration.

use serde::Deserialize;

use super::settings::{default_opacity, opacity_deserializer};

/// Menu-specific appearance configuration.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct MenuAppearance {
    #[serde(deserialize_with = "opacity_deserializer", default = "default_opacity")]
    pub opacity:  f32,
    #[serde(default)]
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
