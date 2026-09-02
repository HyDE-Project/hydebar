//! Root configuration type assembled from the per module definitions.

mod appearance;
mod battery;
mod clock;
mod control_center;
mod custom_module;
mod desk;
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
    MODULE_SIDE_PADDING_EM, MODULE_VERTICAL_PADDING_EM, MenuAppearance, SizeValue,
    WORKSPACE_ACTIVE_MARGIN_EM, WORKSPACE_ACTIVE_PADDING_EM, WORKSPACE_GAP_EM,
    WORKSPACE_GLYPH_ADVANCE_EM, WORKSPACE_MIN_HEIGHT_EM, WORKSPACE_MIN_WIDTH_EM,
    WORKSPACE_PADDING_EM, WindowBorder, WindowShadow
};
pub use battery::BatteryModuleConfig;
pub use clock::ClockModuleConfig;
pub use control_center::ControlCenterModuleConfig;
pub use custom_module::{CustomMenuEntry, CustomModuleDef, CustomModuleSource};
pub use desk::DeskConfig;
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

/// Where the bar looks for its configuration when nothing else names one.
pub const DEFAULT_CONFIG_FILE_PATH: &str = "~/.config/hydebar/config.toml";

/// Complete bar configuration.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(default = "default_log_level")]
    /// How much the bar writes to its log.
    pub log_level:           String,
    #[serde(default)]
    /// Which screen edge the bar stands on.
    pub position:            Position,
    #[serde(default)]
    /// Which compositor layer the bar is drawn in.
    pub layer:               BarLayer,
    #[serde(default)]
    /// Which monitors the bar appears on.
    pub outputs:             Outputs,
    #[serde(default)]
    /// What is drawn in each of the bar's three sections.
    pub modules:             Modules,
    /// The command the launcher entry runs.
    pub app_launcher_cmd:    Option<String>,
    #[serde(rename = "CustomModule", default)]
    /// The modules the user wrote, each named by its own key.
    pub custom_modules:      Vec<CustomModuleDef>,
    /// The command the clipboard entry runs.
    pub clipboard_cmd:       Option<String>,
    #[serde(default)]
    /// How the update count is read, and what installs it.
    pub updates:             Option<UpdatesModuleConfig>,
    #[serde(default)]
    /// How the compositor's workspaces are drawn.
    pub workspaces:          WorkspacesModuleConfig,
    #[serde(default)]
    /// How the focused window is named.
    pub window_title:        WindowTitleConfig,
    #[serde(default)]
    /// What the machine readout samples, and when it warns.
    pub system:              SystemModuleConfig,
    #[serde(default)]
    /// How the charge and the power source are drawn.
    pub battery:             BatteryModuleConfig,
    #[serde(default)]
    /// How the date and time are written.
    pub clock:               ClockModuleConfig,
    #[serde(default)]
    /// How keeping the screen awake behaves.
    pub idle_inhibitor:      IdleInhibitorModuleConfig,
    #[serde(default, alias = "settings")]
    /// What the quick settings hold and how they open.
    pub control_center:      ControlCenterModuleConfig,
    #[serde(default, deserialize_with = "themes::deserialize_theme_or_appearance")]
    /// The palette, the sizes and the motion the bar is drawn with.
    pub appearance:          Appearance,
    #[serde(default)]
    /// How what is playing is named and driven.
    pub media_player:        MediaPlayerModuleConfig,
    #[serde(default)]
    /// How the keyboard layout is named and stepped.
    pub keyboard_layout:     KeyboardLayoutModuleConfig,
    #[serde(default)]
    /// Whether an open menu takes the keyboard.
    pub menu_keyboard_focus: bool,
    #[serde(default)]
    /// What the keyboard is bound to, in the bar and in its menus.
    pub keybindings:         Keybindings,
    #[serde(default)]
    /// Where the weather is read for, and in what units.
    pub weather:             WeatherModuleConfig,
    #[serde(default)]
    /// Which icon theme the bar draws with.
    pub icons:               IconsConfig,
    #[serde(default)]
    /// Where the notices come from and how long they stand.
    pub notifications:       NotificationsConfig,
    #[serde(default)]
    /// How the overview of the screens and their windows is drawn.
    pub desk:                DeskConfig
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
            notifications:       NotificationsConfig::default(),
            desk:                DeskConfig::default()
        }
    }
}

fn default_log_level() -> String {
    "warn".to_owned()
}

const fn default_menu_keyboard_focus() -> bool {
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
