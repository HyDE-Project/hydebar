//! Root configuration type assembled from the per module definitions.

mod appearance;
mod battery;
mod clock;
mod control_center;
mod custom_module;
mod icons;
mod idle_inhibitor;
mod keybindings;
mod keyboard_layout;
mod media_player;
mod modules;
mod notifications;
mod serde_helpers;
mod system_info;
mod themes;
mod updates;
mod validation;
mod weather;
mod window_title;
mod workspaces;

#[cfg(test)]
mod themes_tests;

pub use appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, BAR_PADDING_EM,
    DEFAULT_FONT_SIZE, DEFAULT_RADIUS, GROUP_GAP_EM, ICON_LABEL_GAP_EM, MODULE_GAP_EM,
    MODULE_SIDE_PADDING_EM, MODULE_VERTICAL_PADDING_EM, MenuAppearance, WindowBorder,
    WindowShadow,
    WORKSPACE_ACTIVE_MARGIN_EM, WORKSPACE_ACTIVE_PADDING_EM, WORKSPACE_GAP_EM,
    WORKSPACE_GLYPH_ADVANCE_EM, WORKSPACE_MIN_HEIGHT_EM, WORKSPACE_MIN_WIDTH_EM,
    WORKSPACE_PADDING_EM
};
pub use battery::BatteryModuleConfig;
pub use clock::ClockModuleConfig;
pub use control_center::ControlCenterModuleConfig;
pub use custom_module::{CustomMenuEntry, CustomModuleDef, CustomModuleSource};
pub use icons::IconsConfig;
pub use idle_inhibitor::IdleInhibitorModuleConfig;
pub use keybindings::{GlobalKeybindings, Keybindings, MenuKeybindings};
pub use keyboard_layout::KeyboardLayoutModuleConfig;
pub use media_player::MediaPlayerModuleConfig;
pub use modules::{BarLayer, ModuleDef, ModuleName, Modules, Outputs, Position};
pub use notifications::{NotificationSource, NotificationsConfig};
use serde::Deserialize;
pub use serde_helpers::RegexCfg;
pub use system_info::{
    MemoryFormat, SystemIndicator, SystemInfoCpu, SystemInfoDisk, SystemInfoGpu, SystemInfoMemory,
    SystemInfoTemperature, SystemModuleConfig
};
pub use themes::PresetTheme;
pub use updates::{HydeBranch, UpdatesModuleConfig};
pub use validation::ConfigValidationError;
pub use weather::WeatherModuleConfig;
pub use window_title::{WindowTitleConfig, WindowTitleMode};
pub use workspaces::{WorkspaceVisibilityMode, WorkspacesModuleConfig};

pub const DEFAULT_CONFIG_FILE_PATH: &str = "~/.config/hydebar/config.toml";

/// Complete bar configuration.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level:           String,
    #[serde(default)]
    pub position:            Position,
    #[serde(default)]
    pub layer:               BarLayer,
    #[serde(default)]
    pub outputs:             Outputs,
    #[serde(default)]
    pub modules:             Modules,
    pub app_launcher_cmd:    Option<String>,
    #[serde(rename = "CustomModule", default)]
    pub custom_modules:      Vec<CustomModuleDef>,
    pub clipboard_cmd:       Option<String>,
    #[serde(default)]
    pub updates:             Option<UpdatesModuleConfig>,
    #[serde(default)]
    pub workspaces:          WorkspacesModuleConfig,
    #[serde(default)]
    pub window_title:        WindowTitleConfig,
    #[serde(default)]
    pub system:              SystemModuleConfig,
    #[serde(default)]
    pub battery:             BatteryModuleConfig,
    #[serde(default)]
    pub clock:               ClockModuleConfig,
    #[serde(default)]
    pub idle_inhibitor:      IdleInhibitorModuleConfig,
    #[serde(default, alias = "settings")]
    pub control_center:      ControlCenterModuleConfig,
    #[serde(default, deserialize_with = "themes::deserialize_theme_or_appearance")]
    pub appearance:          Appearance,
    #[serde(default)]
    pub media_player:        MediaPlayerModuleConfig,
    #[serde(default)]
    pub keyboard_layout:     KeyboardLayoutModuleConfig,
    #[serde(default)]
    pub menu_keyboard_focus: bool,
    #[serde(default)]
    pub keybindings:         Keybindings,
    #[serde(default)]
    pub weather:             WeatherModuleConfig,
    #[serde(default)]
    pub icons:               IconsConfig,
    #[serde(default)]
    pub notifications:       NotificationsConfig
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level:           default_log_level(),
            position:            Position::Top,
            layer:               BarLayer::default(),
            outputs:             Outputs::default(),
            modules:             Modules::default(),
            app_launcher_cmd:    Some(default_app_launcher_cmd()),
            clipboard_cmd:       Some(default_clipboard_cmd()),
            updates:             None,
            workspaces:          WorkspacesModuleConfig::default(),
            window_title:        WindowTitleConfig::default(),
            system:              SystemModuleConfig::default(),
            battery:             BatteryModuleConfig::default(),
            clock:               ClockModuleConfig::default(),
            idle_inhibitor:      IdleInhibitorModuleConfig::default(),
            control_center:      ControlCenterModuleConfig::default(),
            appearance:          Appearance::default(),
            media_player:        MediaPlayerModuleConfig::default(),
            keyboard_layout:     KeyboardLayoutModuleConfig::default(),
            custom_modules:      vec![],
            menu_keyboard_focus: default_menu_keyboard_focus(),
            keybindings:         Keybindings::default(),
            weather:             WeatherModuleConfig::default(),
            icons:               IconsConfig::default(),
            notifications:       NotificationsConfig::default()
        }
    }
}

fn default_log_level() -> String {
    "warn".to_owned()
}

fn default_menu_keyboard_focus() -> bool {
    true
}

/// Launcher invoked by the app launcher module when the user has not
/// configured one.
fn default_app_launcher_cmd() -> String {
    "rofi -show drun".to_owned()
}

/// Clipboard history picker invoked by the clipboard module when the user has
/// not configured one.
fn default_clipboard_cmd() -> String {
    "cliphist list | rofi -dmenu | cliphist decode | wl-copy".to_owned()
}
