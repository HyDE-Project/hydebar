//! Configuration for the settings module.

use serde::Deserialize;

/// Settings menu commands and toggles.
#[derive(Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct SettingsModuleConfig {
    pub lock_cmd:               Option<String>,
    #[serde(default = "default_shutdown_cmd")]
    pub shutdown_cmd:           String,
    #[serde(default = "default_suspend_cmd")]
    pub suspend_cmd:            String,
    #[serde(default = "default_reboot_cmd")]
    pub reboot_cmd:             String,
    #[serde(default = "default_logout_cmd")]
    pub logout_cmd:             String,
    pub audio_sinks_more_cmd:   Option<String>,
    pub audio_sources_more_cmd: Option<String>,
    pub wifi_more_cmd:          Option<String>,
    pub vpn_more_cmd:           Option<String>,
    pub bluetooth_more_cmd:     Option<String>,
    #[serde(default)]
    pub remove_airplane_btn:    bool,
    #[serde(default)]
    pub remove_idle_btn:        bool
}

fn default_shutdown_cmd() -> String {
    "shutdown now".to_string()
}

fn default_suspend_cmd() -> String {
    "systemctl suspend".to_string()
}

fn default_reboot_cmd() -> String {
    "systemctl reboot".to_string()
}

fn default_logout_cmd() -> String {
    "loginctl kill-user $(whoami)".to_string()
}
