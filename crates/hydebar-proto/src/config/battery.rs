//! Configuration for the battery module.

use serde::Deserialize;

/// Battery module behaviour.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent configuration switch"
)]
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BatteryModuleConfig {
    #[serde(default = "default_show_percentage")]
    pub show_percentage:        bool,
    #[serde(default = "default_show_power_profile")]
    pub show_power_profile:     bool,
    #[serde(default = "default_open_settings_on_click")]
    pub open_settings_on_click: bool,
    #[serde(default)]
    pub show_when_unavailable:  bool
}

impl Default for BatteryModuleConfig {
    fn default() -> Self {
        Self {
            show_percentage:        default_show_percentage(),
            show_power_profile:     default_show_power_profile(),
            open_settings_on_click: default_open_settings_on_click(),
            show_when_unavailable:  false
        }
    }
}

const fn default_show_percentage() -> bool {
    true
}

const fn default_show_power_profile() -> bool {
    true
}

const fn default_open_settings_on_click() -> bool {
    true
}
