//! Configuration for the keyboard layout module.

use std::collections::HashMap;

use serde::Deserialize;

/// Display labels overriding the compositor reported layout names.
#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct KeyboardLayoutModuleConfig {
    #[serde(default)]
    pub labels: HashMap<String, String>
}
