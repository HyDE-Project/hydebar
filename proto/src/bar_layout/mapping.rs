//! The tables mapping layout entry names onto the bar's own modules.

use std::collections::BTreeSet;

use crate::config::ModuleName;

/// Maps one layout entry onto the module answering for it.
///
/// The `#variant` suffix is dropped first: the layouts use it to pick styling
/// or an alternative definition of the same module, neither of which changes
/// what the module *is*. A `custom/` name whose tail the configuration
/// defines as a custom module maps to that definition: the icon and the
/// command written there are the user's word, and a native module — whatever
/// it does better — has no right to displace the glyphs a person chose. The
/// built-in tables answer for the rest, and a name nothing answers for is
/// skipped, which is how a layout survives entries we have no counterpart
/// for.
pub(super) fn module_for(name: &str, custom: &BTreeSet<&str>) -> Option<ModuleName> {
    let name = name.split('#').next().unwrap_or(name);

    if let Some(tail) = name.strip_prefix("custom/") {
        if matches!(tail, "theme" | "themeswitch") {
            return Some(ModuleName::Themes);
        }

        if tail == "wallchange" {
            return Some(ModuleName::Wallpaper);
        }

        if matches!(tail, "wbar" | "waybar") {
            return Some(ModuleName::BarLayout);
        }

        if tail == "hyde-menu" {
            return Some(ModuleName::HydeMenu);
        }

        if custom.contains(tail) {
            return Some(ModuleName::Custom(tail.to_owned()));
        }

        return builtin_for_custom(tail);
    }

    builtin_for(name)
}

/// The module answering for a plain layout entry.
fn builtin_for(name: &str) -> Option<ModuleName> {
    Some(match name {
        "cpu" => ModuleName::Cpu,
        "memory" => ModuleName::Memory,
        "clock" => ModuleName::Clock,
        "idle_inhibitor" => ModuleName::IdleInhibitor,
        "hyprland/workspaces" | "wlr/workspaces" | "ext/workspaces" => ModuleName::Workspaces,
        "hyprland/window" => ModuleName::WindowTitle,
        "hyprland/language" | "keyboard-state" => ModuleName::KeyboardLayout,
        "hyprland/submap" => ModuleName::KeyboardSubmap,
        "network" => ModuleName::Network,
        "backlight" => ModuleName::Brightness,
        "bluetooth" => ModuleName::Bluetooth,
        "pulseaudio" | "wireplumber" => ModuleName::Audio,
        "battery" => ModuleName::Battery,
        "power-profiles-daemon" => ModuleName::PowerProfile,
        "tray" => ModuleName::Tray,
        "wlr/taskbar" => ModuleName::Taskbar,
        "privacy" => ModuleName::Privacy,
        "mpris" => ModuleName::MediaPlayer,
        "gamemode" => ModuleName::GameMode,
        "image" => ModuleName::Wallpaper,
        _ => return None
    })
}

/// The module answering for a `custom/` layout entry the configuration does
/// not define itself.
fn builtin_for_custom(tail: &str) -> Option<ModuleName> {
    Some(match tail {
        "updates" => ModuleName::Updates,
        "cpuinfo" => ModuleName::Cpu,
        "gpuinfo" => ModuleName::GpuTemp,
        "sensorsinfo" => ModuleName::CpuTemp,
        "keybindhint" | "keybinds_hint" => ModuleName::KeybindHint,
        "weather" => ModuleName::Weather,
        "hyprsunset" => ModuleName::NightLight,
        "gamemode" => ModuleName::GameMode,
        "cliphist" | "clipboard" => ModuleName::Clipboard,
        "power" | "powermenu" => ModuleName::Settings,
        "theme" | "themeswitch" => ModuleName::Themes,
        "wallchange" => ModuleName::Wallpaper,
        "wbar" | "waybar" => ModuleName::BarLayout,
        "hyde-menu" => ModuleName::HydeMenu,
        "spotify" | "mediaplayer" => ModuleName::MediaPlayer,
        "swaync" | "dunst" | "notifications" => ModuleName::Notifications,
        "bluetooth" => ModuleName::Bluetooth,
        "launcher" | "app-launcher" => ModuleName::AppLauncher,
        _ => return None
    })
}
