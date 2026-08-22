//! Configuration for the window title module.

use serde::Deserialize;

/// Whether the module renders the window title or its application class.
#[derive(Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub enum WindowTitleMode {
    /// The title the window carries.
    #[default]
    Title,
    /// The class the application registers under.
    Class
}

/// Window title module behaviour.
#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct WindowTitleConfig {
    /// Whether the window is named by its title or by its class.
    #[serde(default)]
    pub mode: WindowTitleMode,
    /// How many characters are drawn before the name is cut.
    #[serde(default = "default_truncate_title_after_length")]
    pub truncate_title_after_length: u32
}

const fn default_truncate_title_after_length() -> u32 {
    150
}
