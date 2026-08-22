//! The names the bar's modules answer to, and the roster of every module it
//! ships.

/// Named module variants supported by the bar.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModuleName {
    /// Opens the launcher the desktop is configured with.
    AppLauncher,
    /// Counts the packages waiting to be installed, and installs them.
    Updates,
    /// Opens the clipboard history the desktop keeps.
    Clipboard,
    /// Draws the compositor's workspaces and moves between them.
    Workspaces,
    /// Names the window holding focus.
    WindowTitle,
    /// Reads the machine — load, memory, heat, disks and links.
    SystemInfo,
    /// Shows how hard the processor is working.
    Cpu,
    /// Shows how much of the memory is in use.
    Memory,
    /// Shows how hot the processor is running.
    CpuTemp,
    /// Shows how hot the graphics card is running.
    GpuTemp,
    /// Names the keyboard layout in force, and steps through them.
    KeyboardLayout,
    /// Names the compositor submap the keyboard is bound into.
    KeyboardSubmap,
    /// Holds the icons applications register with the system tray.
    Tray,
    /// Lists the open windows and raises the one that is picked.
    Taskbar,
    /// States the date and time, and opens the calendar.
    Clock,
    /// Reads the charge left and how the machine is powered.
    Battery,
    /// Warns while the microphone, the camera or the screen is being read.
    Privacy,
    /// Gathers the quick settings behind one entry.
    ControlCenter,
    /// Reads and sets the volume of the sinks and the sources.
    Audio,
    /// Reads the links, joins a network and holds the VPNs.
    Network,
    /// Reads the adapter and the devices paired with it.
    Bluetooth,
    /// Names the power profile in force, and steps through them.
    PowerProfile,
    /// Reads and sets the backlight.
    Brightness,
    /// Opens the list of what the keyboard is bound to.
    KeybindHint,
    /// Warms the screen after dark.
    NightLight,
    /// Turns the desktop's game mode on and off.
    GameMode,
    /// Reads the weather where the machine stands.
    Weather,
    /// Opens the window the bar is configured from.
    Settings,
    /// Steps and picks the `HyDE` theme in force.
    Themes,
    /// Steps and picks the wallpaper the theme wears.
    Wallpaper,
    /// Steps and picks the `HyDE` bar layout in force.
    BarLayout,
    /// Opens the menu the `HyDE` desktop publishes.
    HydeMenu,
    /// Names what is playing and drives the player.
    MediaPlayer,
    /// Holds the notices that arrived and reopens them.
    Notifications,
    /// Takes a picture of the screen, a window or a region.
    Screenshot,
    /// Keeps the screen awake while it is on.
    IdleInhibitor,
    /// A module the user wrote, named by its own key.
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
