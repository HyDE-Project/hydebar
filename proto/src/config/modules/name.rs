//! The names the bar's modules answer to, and the roster of every module it
//! ships.
//!
//! One declarative list grows all four at once: the [`ModuleName`] variants,
//! the roster of everything shipped, the spelling the configuration writes
//! and the spelling a person reads. Adding a module means adding one row
//! here; the compiler then points at every match that must learn the new
//! arm.

/// Declares the shipped modules once and grows every naming form from the
/// declaration.
///
/// Each row names the variant, carries its documentation and states the
/// human label. The configuration spelling is the variant name itself, so
/// the two spellings can never drift apart.
macro_rules! builtin_modules {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $label:literal
        ),* $(,)?
    ) => {
        /// Named module variants supported by the bar.
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum ModuleName {
            $(
                $(#[$meta])*
                $variant,
            )*
            /// A module the user wrote, named by its own key.
            Custom(String)
        }

        impl ModuleName {
            /// Every module the bar ships, in the order the editor lists them.
            pub const BUILT_IN: &[Self] = &[$(Self::$variant),*];

            /// Name this module is written as in the configuration.
            #[must_use]
            pub const fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => stringify!($variant),)*
                    Self::Custom(name) => name.as_str()
                }
            }

            /// Name this module is shown to a person under.
            ///
            /// The configuration spelling doubles as an identifier and reads
            /// like one; this is the spelling for surfaces a user looks at,
            /// such as the hint shown while the pointer rests on a module.
            #[must_use]
            pub const fn label(&self) -> &str {
                match self {
                    $(Self::$variant => $label,)*
                    Self::Custom(name) => name.as_str()
                }
            }
        }
    };
}

builtin_modules! {
    /// Opens the launcher the desktop is configured with.
    AppLauncher => "App launcher",
    /// Counts the packages waiting to be installed, and installs them.
    Updates => "Updates",
    /// Opens the clipboard history the desktop keeps.
    Clipboard => "Clipboard",
    /// Draws the compositor's workspaces and moves between them.
    Workspaces => "Workspaces",
    /// Names the window holding focus.
    WindowTitle => "Window title",
    /// Reads the machine — load, memory, heat, disks and links.
    SystemInfo => "System monitor",
    /// Shows how hard the processor is working.
    Cpu => "Processor",
    /// Shows how much of the memory is in use.
    Memory => "Memory",
    /// Shows how hot the processor is running.
    CpuTemp => "CPU temperature",
    /// Shows how hot the graphics card is running.
    GpuTemp => "GPU temperature",
    /// Names the keyboard layout in force, and steps through them.
    KeyboardLayout => "Keyboard layout",
    /// Names the compositor submap the keyboard is bound into.
    KeyboardSubmap => "Keyboard submap",
    /// Holds the icons applications register with the system tray.
    Tray => "Tray",
    /// Lists the open windows and raises the one that is picked.
    Taskbar => "Taskbar",
    /// States the date and time, and opens the calendar.
    Clock => "Clock",
    /// Reads the charge left and how the machine is powered.
    Battery => "Battery",
    /// Warns while the microphone, the camera or the screen is being read.
    Privacy => "Privacy",
    /// Gathers the quick settings behind one entry.
    ControlCenter => "Control centre",
    /// Reads and sets the volume of the sinks and the sources.
    Audio => "Audio",
    /// Reads the links, joins a network and holds the VPNs.
    Network => "Network",
    /// Reads the adapter and the devices paired with it.
    Bluetooth => "Bluetooth",
    /// Names the power profile in force, and steps through them.
    PowerProfile => "Power profile",
    /// Reads and sets the backlight.
    Brightness => "Brightness",
    /// Opens the list of what the keyboard is bound to.
    KeybindHint => "Key bindings",
    /// Warms the screen after dark.
    NightLight => "Night light",
    /// Turns the desktop's game mode on and off.
    GameMode => "Game mode",
    /// Reads the weather where the machine stands.
    Weather => "Weather",
    /// Opens the window the bar is configured from.
    Settings => "Bar settings",
    /// Steps and picks the `HyDE` theme in force.
    Themes => "Desktop themes",
    /// Steps and picks the wallpaper the theme wears.
    Wallpaper => "Wallpaper",
    /// Steps and picks the `HyDE` bar layout in force.
    BarLayout => "Bar layout",
    /// Opens the menu the `HyDE` desktop publishes.
    HydeMenu => "HyDE menu",
    /// Names what is playing and drives the player.
    MediaPlayer => "Media player",
    /// Holds the notices that arrived and reopens them.
    Notifications => "Notifications",
    /// Takes a picture of the screen, a window or a region.
    Screenshot => "Screenshot",
    /// Keeps the screen awake while it is on.
    IdleInhibitor => "Idle inhibitor"
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

    /// A module a person cannot name is a module a person cannot find; every
    /// shipped module carries a spelled-out label distinct from its
    /// configuration name style.
    #[test]
    fn every_built_in_module_carries_a_human_label() {
        for module in ModuleName::BUILT_IN {
            assert!(!module.label().is_empty());
        }

        assert_eq!(ModuleName::PowerProfile.label(), "Power profile");
        assert_eq!(ModuleName::Custom("memory".to_owned()).label(), "memory");
    }

    /// The configuration spelling is the variant name itself: the roster and
    /// the reader agree without a hand-kept table between them.
    #[test]
    fn configuration_spellings_match_their_variants() {
        assert_eq!(ModuleName::Cpu.as_str(), "Cpu");
        assert_eq!(ModuleName::HydeMenu.as_str(), "HydeMenu");
        assert_eq!(ModuleName::IdleInhibitor.as_str(), "IdleInhibitor");
        assert_eq!(ModuleName::Custom("demo".to_owned()).as_str(), "demo");
    }
}
