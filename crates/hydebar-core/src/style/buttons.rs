//! Button styles used across the bar.

mod module;
mod plain;
mod quick;
mod workspace;

#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests;

pub use module::module_button_style;
pub use plain::{
    confirm_button_style, ghost_button_style, menu_entry_button_style, outline_button_style,
    settings_button_style
};
pub use quick::{quick_settings_button_style, quick_settings_submenu_button_style};
pub use workspace::workspace_button_style;
