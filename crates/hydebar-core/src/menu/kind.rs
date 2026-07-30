//! The menus a bar module can open.

use hydebar_proto::config::ModuleName;

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum MenuType {
    Updates,
    ControlCenter,
    /// Menu of the standalone audio module.
    Audio,
    /// Menu of the standalone network module.
    Network,
    /// Menu of the standalone bluetooth module.
    Bluetooth,
    /// Menu of the standalone power profile module.
    PowerProfile,
    /// Window configuring the bar itself.
    Settings,
    HydeMenu,
    /// Menu of the theme module, listing the installed desktop themes.
    Themes,
    Tray(String),
    MediaPlayer,
    SystemInfo,
    Notifications,
    Screenshot,
    Calendar,
    /// Context menu of the custom module carrying the given name.
    Custom(String)
}

impl MenuType {
    /// The bar module the menu was opened from.
    ///
    /// Opening a menu is the user looking at the module it belongs to, so the
    /// bar reads the attention out of the menu the same way it reads it out of
    /// the pointer, and a menu left open keeps its module attended after the
    /// pointer has moved on.
    #[must_use]
    pub fn owner(&self) -> ModuleName {
        match self {
            Self::Updates => ModuleName::Updates,
            Self::ControlCenter => ModuleName::ControlCenter,
            Self::Audio => ModuleName::Audio,
            Self::Network => ModuleName::Network,
            Self::Bluetooth => ModuleName::Bluetooth,
            Self::PowerProfile => ModuleName::PowerProfile,
            Self::Settings => ModuleName::Settings,
            Self::HydeMenu => ModuleName::HydeMenu,
            Self::Themes => ModuleName::Themes,
            Self::Tray(_) => ModuleName::Tray,
            Self::MediaPlayer => ModuleName::MediaPlayer,
            Self::SystemInfo => ModuleName::SystemInfo,
            Self::Notifications => ModuleName::Notifications,
            Self::Screenshot => ModuleName::Screenshot,
            Self::Calendar => ModuleName::Clock,
            Self::Custom(name) => ModuleName::Custom(name.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_menu_names_the_module_it_belongs_to() {
        assert_eq!(MenuType::Network.owner(), ModuleName::Network);
        assert_eq!(MenuType::Calendar.owner(), ModuleName::Clock);
        assert_eq!(
            MenuType::Custom("power".to_owned()).owner(),
            ModuleName::Custom("power".to_owned())
        );
    }

    #[test]
    fn the_theme_menu_belongs_to_the_theme_module() {
        assert_eq!(MenuType::Themes.owner(), ModuleName::Themes);
    }
}
