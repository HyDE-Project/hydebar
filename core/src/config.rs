//! The bar configuration: how it is found, read, overlaid and kept fresh.
//!
//! Loading the file lives in [`load`], reading and parsing it in [`read`],
//! the `HyDE` desktop overlay in [`hyde`], the reload state and its impact in
//! [`manager`], and the file and theme watchers in [`watch`] and
//! [`theme_watch`].

/// The configuration types core's own surface speaks in, re-exported by
/// name from the domain crate — one deliberate list instead of a glob, so
/// a new domain type never gains a second import path unreviewed.
pub use hydebar_proto::config::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, BarLayer, BatteryModuleConfig,
    ClockModuleConfig, Config, ControlCenterModuleConfig, CustomMenuEntry, CustomModuleDef,
    DEFAULT_CONFIG_FILE_PATH, DEFAULT_RADIUS, HydeBranch, KeyboardLayoutModuleConfig,
    MODULE_VERTICAL_PADDING_EM, MediaPlayerModuleConfig, MemoryFormat, MenuAppearance, ModuleDef,
    ModuleName, Modules, NotificationSource, Outputs, Position, SystemIndicator,
    SystemModuleConfig, UpdatesModuleConfig, WORKSPACE_ACTIVE_MARGIN_EM,
    WORKSPACE_ACTIVE_PADDING_EM, WORKSPACE_GAP_EM, WORKSPACE_GLYPH_ADVANCE_EM,
    WORKSPACE_MIN_HEIGHT_EM, WORKSPACE_MIN_WIDTH_EM, WORKSPACE_PADDING_EM, WindowTitleConfig,
    WindowTitleMode, WorkspaceVisibilityMode, WorkspacesModuleConfig
};

mod hyde;
mod load;
mod read;

pub mod manager;
pub mod theme_watch;
pub mod watch;

pub use load::{ConfigLoadError, get_config};
pub use manager::{
    ConfigApplied, ConfigDegradation, ConfigImpact, ConfigManager, ConfigUpdateError
};
pub(crate) use read::{ConfigReadError, read_config, read_config_with};
pub use theme_watch::{ThemeRoots, ThemeWatchTarget, theme_subscription};
pub use watch::{ConfigEvent, subscription};
