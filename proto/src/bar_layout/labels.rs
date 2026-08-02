//! Spelling configured module names the way a person would write them.

/// States a configured module name the way a person would write it.
///
/// The names `HyDE` ships its helper scripts under are terse identifiers; a
/// hint reading `wbar` or `hyde-menu` looks like debugging output next to
/// `Battery: 85%`. The names the scripts are known by get the label a person
/// would use, and anything else is at least spelled like a word: first letter
/// up, separators as spaces.
#[must_use]
pub fn display_label(name: &str) -> String {
    let known = match name {
        "cliphist" | "clipboard" => "Clipboard",
        "wallchange" => "Wallpaper",
        "theme" | "themeswitch" => "Theme switcher",
        "wbar" => "Bar layout",
        "hyde-menu" => "HyDE menu",
        "keybindhint" | "keybinds_hint" => "Key bindings",
        "power" | "powermenu" => "Power menu",
        "cpuinfo" => "Processor",
        "gpuinfo" => "Graphics",
        "memory" => "Memory",
        "sensorsinfo" => "Sensors",
        "hyprsunset" => "Night light",
        "gamemode" => "Game mode",
        "cava" => "Audio visualiser",
        "spotify" => "Media player",
        "swaync" | "dunst" => "Notifications",
        "weather" => "Weather",
        _ => ""
    };

    if !known.is_empty() {
        return known.to_owned();
    }

    let spaced = name.replace(['-', '_'], " ");
    let mut chars = spaced.chars();

    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    /// A hint is read next to `Battery: 85%`; a raw identifier beside that
    /// reads as a bug, so every configured name comes out spelled for people.
    #[test]
    fn configured_names_are_spelled_for_people() {
        assert_eq!(display_label("wbar"), "Bar layout");
        assert_eq!(display_label("hyde-menu"), "HyDE menu");
        assert_eq!(display_label("keybindhint"), "Key bindings");
        assert_eq!(display_label("my-own_module"), "My own module");
    }
}
