//! The names the bar's modules answer to, and the roster of every module it
//! ships.

/// Named module variants supported by the bar.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModuleName {
    AppLauncher,
    Updates,
    Clipboard,
    Workspaces,
    WindowTitle,
    SystemInfo,
    Cpu,
    Memory,
    CpuTemp,
    GpuTemp,
    KeyboardLayout,
    KeyboardSubmap,
    Tray,
    Taskbar,
    Clock,
    Battery,
    Privacy,
    ControlCenter,
    Audio,
    Network,
    Bluetooth,
    PowerProfile,
    Brightness,
    KeybindHint,
    NightLight,
    GameMode,
    Weather,
    Settings,
    Themes,
    Wallpaper,
    /// Steps and picks the `HyDE` bar layout in force.
    BarLayout,
    HydeMenu,
    MediaPlayer,
    Notifications,
    Screenshot,
    IdleInhibitor,
    Custom(String)
}

impl ModuleName {
    /// Every module the bar ships, in the order the editor lists them.
    pub const BUILT_IN: [Self; 36] = [
        Self::AppLauncher,
        Self::Updates,
        Self::Clipboard,
        Self::Workspaces,
        Self::WindowTitle,
        Self::SystemInfo,
        Self::Cpu,
        Self::Memory,
        Self::CpuTemp,
        Self::GpuTemp,
        Self::KeyboardLayout,
        Self::KeyboardSubmap,
        Self::Tray,
        Self::Taskbar,
        Self::Clock,
        Self::Battery,
        Self::Privacy,
        Self::ControlCenter,
        Self::Audio,
        Self::Network,
        Self::Bluetooth,
        Self::PowerProfile,
        Self::Brightness,
        Self::KeybindHint,
        Self::NightLight,
        Self::GameMode,
        Self::Weather,
        Self::Settings,
        Self::Themes,
        Self::Wallpaper,
        Self::BarLayout,
        Self::HydeMenu,
        Self::MediaPlayer,
        Self::Notifications,
        Self::Screenshot,
        Self::IdleInhibitor
    ];
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    /// A module the editor cannot offer is a module nobody can place from the
    /// window, whatever else it can do.
    #[test]
    fn the_theme_module_is_one_the_layout_editor_offers() {
        assert!(ModuleName::BUILT_IN.contains(&ModuleName::Themes));
    }
}
