//! The two spellings of a module name: the one the configuration writes and
//! the one a person reads.

use super::name::ModuleName;

impl ModuleName {
    /// Name this module is written as in the configuration.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::AppLauncher => "AppLauncher",
            Self::Updates => "Updates",
            Self::Clipboard => "Clipboard",
            Self::Workspaces => "Workspaces",
            Self::WindowTitle => "WindowTitle",
            Self::SystemInfo => "SystemInfo",
            Self::Cpu => "Cpu",
            Self::Memory => "Memory",
            Self::CpuTemp => "CpuTemp",
            Self::GpuTemp => "GpuTemp",
            Self::KeyboardLayout => "KeyboardLayout",
            Self::KeyboardSubmap => "KeyboardSubmap",
            Self::Tray => "Tray",
            Self::Taskbar => "Taskbar",
            Self::Clock => "Clock",
            Self::Battery => "Battery",
            Self::Privacy => "Privacy",
            Self::ControlCenter => "ControlCenter",
            Self::Audio => "Audio",
            Self::Network => "Network",
            Self::Bluetooth => "Bluetooth",
            Self::PowerProfile => "PowerProfile",
            Self::Brightness => "Brightness",
            Self::KeybindHint => "KeybindHint",
            Self::NightLight => "NightLight",
            Self::GameMode => "GameMode",
            Self::Weather => "Weather",
            Self::Settings => "Settings",
            Self::Themes => "Themes",
            Self::Wallpaper => "Wallpaper",
            Self::BarLayout => "BarLayout",
            Self::HydeMenu => "HydeMenu",
            Self::MediaPlayer => "MediaPlayer",
            Self::Notifications => "Notifications",
            Self::Screenshot => "Screenshot",
            Self::IdleInhibitor => "IdleInhibitor",
            Self::Custom(name) => name.as_str()
        }
    }

    /// Name this module is shown to a person under.
    ///
    /// The configuration spelling doubles as an identifier and reads like one;
    /// this is the spelling for surfaces a user looks at, such as the hint
    /// shown while the pointer rests on a module.
    #[must_use]
    pub const fn label(&self) -> &str {
        match self {
            Self::AppLauncher => "App launcher",
            Self::Updates => "Updates",
            Self::Clipboard => "Clipboard",
            Self::Workspaces => "Workspaces",
            Self::WindowTitle => "Window title",
            Self::SystemInfo => "System monitor",
            Self::Cpu => "Processor",
            Self::Memory => "Memory",
            Self::CpuTemp => "CPU temperature",
            Self::GpuTemp => "GPU temperature",
            Self::KeyboardLayout => "Keyboard layout",
            Self::KeyboardSubmap => "Keyboard submap",
            Self::Tray => "Tray",
            Self::Taskbar => "Taskbar",
            Self::Clock => "Clock",
            Self::Battery => "Battery",
            Self::Privacy => "Privacy",
            Self::ControlCenter => "Control centre",
            Self::Audio => "Audio",
            Self::Network => "Network",
            Self::Bluetooth => "Bluetooth",
            Self::PowerProfile => "Power profile",
            Self::Brightness => "Brightness",
            Self::KeybindHint => "Key bindings",
            Self::NightLight => "Night light",
            Self::GameMode => "Game mode",
            Self::Weather => "Weather",
            Self::Settings => "Bar settings",
            Self::Themes => "Desktop themes",
            Self::Wallpaper => "Wallpaper",
            Self::BarLayout => "Bar layout",
            Self::HydeMenu => "HyDE menu",
            Self::MediaPlayer => "Media player",
            Self::Notifications => "Notifications",
            Self::Screenshot => "Screenshot",
            Self::IdleInhibitor => "Idle inhibitor",
            Self::Custom(name) => name.as_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A module a person cannot name is a module a person cannot find; every
    /// shipped module carries a spelled-out label distinct from its
    /// configuration name style.
    #[test]
    fn every_built_in_module_carries_a_human_label() {
        for module in &ModuleName::BUILT_IN {
            assert!(!module.label().is_empty());
        }

        assert_eq!(ModuleName::PowerProfile.label(), "Power profile");
        assert_eq!(ModuleName::Custom("memory".to_owned()).label(), "memory");
    }
}
