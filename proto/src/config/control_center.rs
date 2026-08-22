//! Configuration for the settings module.

use serde::Deserialize;

/// Settings menu commands and toggles.
#[derive(Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ControlCenterModuleConfig {
    /// The command that locks the session.
    pub lock_cmd:               Option<String>,
    #[serde(default = "default_shutdown_cmd")]
    /// The command that powers the machine off.
    pub shutdown_cmd:           String,
    #[serde(default = "default_suspend_cmd")]
    /// The command that suspends the machine.
    pub suspend_cmd:            String,
    #[serde(default = "default_reboot_cmd")]
    /// The command that restarts the machine.
    pub reboot_cmd:             String,
    #[serde(default = "default_logout_cmd")]
    /// The command that ends the session.
    pub logout_cmd:             String,
    /// The command opening the full output settings.
    pub audio_sinks_more_cmd:   Option<String>,
    /// The command opening the full input settings.
    pub audio_sources_more_cmd: Option<String>,
    /// The command opening the full network settings.
    pub wifi_more_cmd:          Option<String>,
    /// The command opening the full VPN settings.
    pub vpn_more_cmd:           Option<String>,
    /// The command opening the full bluetooth settings.
    pub bluetooth_more_cmd:     Option<String>,
    #[serde(default)]
    /// Whether the airplane mode button is left out.
    pub remove_airplane_btn:    bool,
    #[serde(default)]
    /// Whether the keep-awake button is left out.
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
