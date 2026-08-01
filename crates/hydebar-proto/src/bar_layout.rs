//! The active `HyDE` bar layout, restated in this bar's modules.
//!
//! `HyDE` arranges its session bar from interchangeable layout files: three
//! position arrays whose entries are either module names or the names of pill
//! groups declared beside them. The layout in force is recorded in `staterc`,
//! and switching layouts is nothing but rewriting that record — which this
//! bar already watches. Taking the seat therefore takes no new machinery:
//! read the recorded layout, restate it in our modules, and let the existing
//! reload apply it. The names inside the files, and the `WAYBAR_` spelling of
//! the state keys, are the vocabulary of the bar `HyDE` shipped with before
//! this one; they are read as found and answered with our own modules.
//!
//! The restatement only answers for a configuration that wrote no module
//! layout of its own. A hand-written `[modules]` section is the user's word
//! and outranks whatever `HyDE` has on file, exactly as every other setting
//! does.

use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

use crate::{
    config::{ModuleDef, ModuleName, Modules},
    hyde_dirs::HydeDirs,
    shell_vars
};

/// Key under which `staterc` records the layout file in force.
const LAYOUT_PATH_KEY: &str = "WAYBAR_LAYOUT_PATH";

/// Key under which `staterc` records the layout's name.
const LAYOUT_NAME_KEY: &str = "WAYBAR_LAYOUT_NAME";

/// Position arrays a layout may carry, in the order the bar draws them.
const SECTIONS: [&str; 3] = ["modules-left", "modules-center", "modules-right"];

/// Reads the active layout from the user's own directories.
///
/// `custom_names` are the custom modules the bar configuration defines; a
/// layout entry naming one of them is placed as that module, so a user's own
/// wrapper wins over the built-in the name would otherwise map to.
#[must_use]
pub fn load(custom_names: &[String]) -> Option<Modules> {
    HydeDirs::from_env().and_then(|dirs| load_from(&dirs, custom_names))
}

/// Reads the active layout from an explicit `HyDE` install.
///
/// Returns [`None`] whenever any link of the chain is missing — no `staterc`,
/// no readable layout, or a layout none of whose entries the bar can place —
/// and the caller keeps the layout it already has.
#[must_use]
pub fn load_from(dirs: &HydeDirs, custom_names: &[String]) -> Option<Modules> {
    let staterc = fs::read_to_string(dirs.staterc()).ok()?;
    let source = fs::read_to_string(layout_file(dirs, &staterc)?).ok()?;

    parse(&source, custom_names)
}

/// Resolves the layout file `staterc` points at.
///
/// The recorded path wins; when it is gone the recorded *name* is looked up
/// under the shipped layouts, which is also what `HyDE`'s own tooling falls
/// back to.
fn layout_file(dirs: &HydeDirs, staterc: &str) -> Option<PathBuf> {
    if let Some(path) = shell_vars::value_of(staterc, LAYOUT_PATH_KEY) {
        let path = PathBuf::from(path);

        if path.is_file() {
            return Some(path);
        }
    }

    let name = shell_vars::value_of(staterc, LAYOUT_NAME_KEY)?;
    let path = dirs.bar_layouts_dir().join(format!("{name}.jsonc"));

    path.is_file().then_some(path)
}

/// Restates a layout source in the bar's own modules.
///
/// Returns [`None`] for a source that does not parse or maps to nothing at
/// all: an empty bar helps nobody, and the default layout is the better
/// answer to a layout the bar cannot restate.
#[must_use]
pub fn parse(source: &str, custom_names: &[String]) -> Option<Modules> {
    let root: Value = serde_json::from_str(&plain_json(source)).ok()?;
    let custom: BTreeSet<&str> = custom_names.iter().map(String::as_str).collect();
    let mut placed = BTreeSet::new();

    let mut sections = SECTIONS
        .iter()
        .map(|section| section_defs(&root, section, &custom, &mut placed));

    let modules = Modules {
        left:   sections.next()?,
        center: sections.next()?,
        right:  sections.next()?
    };

    (modules.placed().count() > 0).then_some(modules)
}

/// Restates one position array of the layout.
fn section_defs(
    root: &Value,
    section: &str,
    custom: &BTreeSet<&str>,
    placed: &mut BTreeSet<ModuleName>
) -> Vec<ModuleDef> {
    let Some(entries) = root.get(section).and_then(Value::as_array) else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|entry| entry_def(root, entry, custom, placed))
        .collect()
}

/// Restates one entry of a position array.
///
/// A group becomes an island holding its members; a bare name becomes a
/// standalone module. A group whose members all fell away is dropped whole.
fn entry_def(
    root: &Value,
    entry: &str,
    custom: &BTreeSet<&str>,
    placed: &mut BTreeSet<ModuleName>
) -> Option<ModuleDef> {
    if entry.starts_with("group/") {
        let members: Vec<ModuleName> = root
            .get(entry)
            .and_then(|group| group.get("modules"))
            .and_then(Value::as_array)?
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|member| place(member, custom, placed))
            .collect();

        return match members.len() {
            0 => None,
            1 => Some(ModuleDef::Single(members.into_iter().next()?)),
            _ => Some(ModuleDef::Group(members))
        };
    }

    place(entry, custom, placed).map(ModuleDef::Single)
}

/// Maps one layout entry onto a module of ours, at most once per layout.
///
/// Several layout entries can land on the same module — the speaker and
/// microphone are one audio module here — and placing it twice would draw it
/// twice. The first entry wins the spot; later ones fall away.
fn place(
    name: &str,
    custom: &BTreeSet<&str>,
    placed: &mut BTreeSet<ModuleName>
) -> Option<ModuleName> {
    let module = module_for(name, custom)?;

    placed.insert(module.clone()).then_some(module)
}

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
fn module_for(name: &str, custom: &BTreeSet<&str>) -> Option<ModuleName> {
    let name = name.split('#').next().unwrap_or(name);

    if let Some(tail) = name.strip_prefix("custom/") {
        if matches!(tail, "theme" | "themeswitch") {
            return Some(ModuleName::Themes);
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
        "hyprland/workspaces" => ModuleName::Workspaces,
        "hyprland/window" => ModuleName::WindowTitle,
        "hyprland/language" | "keyboard-state" => ModuleName::KeyboardLayout,
        "hyprland/submap" => ModuleName::KeyboardSubmap,
        "network" => ModuleName::Network,
        "bluetooth" => ModuleName::Bluetooth,
        "pulseaudio" | "wireplumber" => ModuleName::Audio,
        "battery" => ModuleName::Battery,
        "power-profiles-daemon" => ModuleName::PowerProfile,
        "tray" => ModuleName::Tray,
        "privacy" => ModuleName::Privacy,
        "mpris" => ModuleName::MediaPlayer,
        _ => return None
    })
}

/// The module answering for a `custom/` layout entry the configuration does
/// not define itself.
fn builtin_for_custom(tail: &str) -> Option<ModuleName> {
    Some(match tail {
        "updates" => ModuleName::Updates,
        "cliphist" | "clipboard" => ModuleName::Clipboard,
        "power" | "powermenu" => ModuleName::Settings,
        "theme" | "themeswitch" => ModuleName::Themes,
        "wallchange" => ModuleName::Wallpaper,
        "hyde-menu" => ModuleName::HydeMenu,
        "spotify" | "mediaplayer" => ModuleName::MediaPlayer,
        "swaync" | "dunst" | "notifications" => ModuleName::Notifications,
        "bluetooth" => ModuleName::Bluetooth,
        "launcher" | "app-launcher" => ModuleName::AppLauncher,
        _ => return None
    })
}

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

    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => spaced
    }
}

/// Strips the relaxed syntax the layout files carry down to plain JSON.
///
/// Layout files carry line and block comments and the odd trailing comma;
/// both are removed outside of strings so the strict parser can take the
/// rest.
#[must_use]
pub fn plain_json(source: &str) -> String {
    let without_comments = strip_comments(source);

    strip_trailing_commas(&without_comments)
}

/// Removes line and block comments, leaving string contents alone.
fn strip_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if in_string {
            output.push(c);
            match c {
                '\\' if !escaped => escaped = true,
                '"' if !escaped => in_string = false,
                _ => escaped = false
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                output.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        output.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = ' ';
                for skipped in chars.by_ref() {
                    if last == '*' && skipped == '/' {
                        break;
                    }
                    last = skipped;
                }
            }
            _ => output.push(c)
        }
    }

    output
}

/// Removes commas left dangling before a closing brace or bracket.
fn strip_trailing_commas(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut in_string = false;
    let mut escaped = false;

    for c in source.chars() {
        if in_string {
            output.push(c);
            match c {
                '\\' if !escaped => escaped = true,
                '"' if !escaped => in_string = false,
                _ => escaped = false
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                output.push(c);
            }
            '}' | ']' => {
                while output.trim_end().ends_with(',') {
                    let end = output.trim_end().len();
                    output.truncate(end - 1);
                }
                output.push(c);
            }
            _ => output.push(c)
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape every `HyDE` layout ships: groups declared beside the position
    /// arrays that reference them, comments included.
    const LAYOUT: &str = r#"
    // vim:ft=jsonc
    {
        "layer": "top",
        "modules-left": ["group/pill#left1", "group/pill#left2"],
        "modules-center": ["group/pill#center"],
        "modules-right": ["group/pill#right1"],
        "group/pill#left1": { "orientation": "inherit", "modules": ["cpu", "memory"] },
        "group/pill#left2": { "orientation": "inherit", "modules": ["idle_inhibitor", "clock"] },
        "group/pill#center": { "orientation": "inherit", "modules": ["hyprland/workspaces", "hyprland/window"] },
        "group/pill#right1": {
            "orientation": "inherit",
            "modules": ["pulseaudio", "pulseaudio#microphone", "custom/updates", "custom/keybindhint"],
        },
        "include": ["~/.config/waybar/includes/includes.json"]
    }
    "#;

    #[test]
    fn a_hyde_layout_restates_as_islands() {
        let custom = vec!["keybindhint".to_owned()];
        let modules = parse(LAYOUT, &custom).expect("layout");

        assert_eq!(
            modules.left,
            vec![
                ModuleDef::Group(vec![ModuleName::Cpu, ModuleName::Memory]),
                ModuleDef::Group(vec![ModuleName::IdleInhibitor, ModuleName::Clock]),
            ]
        );
        assert_eq!(
            modules.center,
            vec![ModuleDef::Group(vec![
                ModuleName::Workspaces,
                ModuleName::WindowTitle,
            ])]
        );
        assert_eq!(
            modules.right,
            vec![ModuleDef::Group(vec![
                ModuleName::Audio,
                ModuleName::Updates,
                ModuleName::Custom("keybindhint".to_owned()),
            ])]
        );
    }

    /// The speaker and microphone entries are one audio module here, placed
    /// once as the group it appears in first; the processor and memory
    /// entries stay two separate modules.
    #[test]
    fn entries_sharing_a_bar_module_are_placed_once() {
        let modules = parse(LAYOUT, &[]).expect("layout");
        let processor = modules
            .placed()
            .filter(|name| **name == ModuleName::Cpu)
            .count();
        let memory = modules
            .placed()
            .filter(|name| **name == ModuleName::Memory)
            .count();
        let audio = modules
            .placed()
            .filter(|name| **name == ModuleName::Audio)
            .count();

        assert_eq!(processor, 1);
        assert_eq!(memory, 1);
        assert_eq!(audio, 1);
    }

    /// The icon and command a user wrote for a name are their word; the
    /// built-in map answers only for names the configuration says nothing
    /// about.
    #[test]
    fn a_defined_custom_module_wins_over_the_built_in_map() {
        let custom = vec!["wallchange".to_owned()];
        let source = r#"{ "modules-left": ["custom/wallchange", "custom/updates"] }"#;

        let modules = parse(source, &custom).expect("layout");

        assert_eq!(
            modules.left,
            vec![
                ModuleDef::Single(ModuleName::Custom("wallchange".to_owned())),
                ModuleDef::Single(ModuleName::Updates),
            ]
        );
    }

    /// Pressing the theme chip has to open the list of themes. A configured
    /// wrapper of that name runs a switch command instead — no list, no
    /// feedback — so the theme chip is the one entry the native module always
    /// answers for.
    #[test]
    fn the_theme_chip_always_opens_the_theme_list() {
        let custom = vec!["theme".to_owned()];
        let source = r#"{ "modules-left": ["custom/theme"] }"#;

        let modules = parse(source, &custom).expect("layout");

        assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Themes)]);
    }

    #[test]
    fn a_bare_module_list_needs_no_groups() {
        let source = r#"{
            "modules-left": ["custom/launcher", "hyprland/window"],
            "modules-right": ["clock"]
        }"#;

        let modules = parse(source, &[]).expect("layout");

        assert_eq!(
            modules.left,
            vec![
                ModuleDef::Single(ModuleName::AppLauncher),
                ModuleDef::Single(ModuleName::WindowTitle),
            ]
        );
        assert!(modules.center.is_empty());
        assert_eq!(modules.right, vec![ModuleDef::Single(ModuleName::Clock)]);
    }

    /// Names the bar has no counterpart for fall away without taking their
    /// group down with them; a group left empty disappears whole.
    #[test]
    fn unknown_names_fall_away_and_an_emptied_group_disappears() {
        let source = r#"{
            "modules-left": ["group/pill#a", "group/pill#b"],
            "group/pill#a": { "modules": ["backlight", "wlr/taskbar"] },
            "group/pill#b": { "modules": ["backlight", "clock"] }
        }"#;

        let modules = parse(source, &[]).expect("layout");

        assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Clock)]);
    }

    /// A layout mapping to nothing at all is refused so the caller keeps a
    /// layout that draws something.
    #[test]
    fn a_layout_the_bar_cannot_restate_is_refused() {
        assert_eq!(parse(r#"{ "modules-left": ["backlight"] }"#, &[]), None);
        assert_eq!(parse("not json at all", &[]), None);
    }

    /// A hint is read next to `Battery: 85%`; a raw identifier beside that
    /// reads as a bug, so every configured name comes out spelled for people.
    #[test]
    fn configured_names_are_spelled_for_people() {
        assert_eq!(display_label("wbar"), "Bar layout");
        assert_eq!(display_label("hyde-menu"), "HyDE menu");
        assert_eq!(display_label("keybindhint"), "Key bindings");
        assert_eq!(display_label("my-own_module"), "My own module");
    }

    #[test]
    fn variant_suffixes_pick_the_same_module() {
        let source = r#"{ "modules-left": ["clock#date", "network#bandwidthUpBytes"] }"#;

        let modules = parse(source, &[]).expect("layout");

        assert_eq!(
            modules.left,
            vec![
                ModuleDef::Single(ModuleName::Clock),
                ModuleDef::Single(ModuleName::Network),
            ]
        );
    }

    #[test]
    fn comments_and_trailing_commas_are_tolerated() {
        let source = "{\n /* block */ \"modules-left\": [\"clock\",], // line\n}";

        let modules = parse(source, &[]).expect("layout");

        assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Clock)]);
    }

    #[test]
    fn comment_markers_inside_strings_are_left_alone() {
        let source = r#"{ "modules-left": ["clock"], "note": "https://example.com/a" }"#;

        assert!(parse(source, &[]).is_some());
    }

    mod loading {
        use std::fs;

        use tempfile::TempDir;

        use super::*;

        fn install(staterc: &str, layout: Option<(&str, &str)>) -> (TempDir, HydeDirs) {
            let root = TempDir::new().expect("tempdir");
            let dirs = HydeDirs::new(
                root.path().join("config"),
                root.path().join("state"),
                root.path().join("cache"),
                root.path().join("data")
            );

            fs::create_dir_all(dirs.hyde_state_dir()).expect("state dir");
            fs::write(dirs.staterc(), staterc).expect("staterc");

            if let Some((relative, source)) = layout {
                let path = dirs.bar_layouts_dir().join(relative);
                fs::create_dir_all(path.parent().expect("parent")).expect("layout dir");
                fs::write(path, source).expect("layout");
            }

            (root, dirs)
        }

        #[test]
        fn the_recorded_path_is_read_first() {
            let (root, dirs) = install("", None);
            let path = root.path().join("somewhere.jsonc");
            fs::write(&path, r#"{ "modules-left": ["clock"] }"#).expect("layout");
            fs::write(
                dirs.staterc(),
                format!("WAYBAR_LAYOUT_PATH=\"{}\"\n", path.display())
            )
            .expect("staterc");

            let modules = load_from(&dirs, &[]).expect("layout");

            assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Clock)]);
        }

        #[test]
        fn a_gone_path_falls_back_to_the_recorded_name() {
            let (_root, dirs) = install(
                "WAYBAR_LAYOUT_PATH=\"/nonexistent/gone.jsonc\"\nWAYBAR_LAYOUT_NAME=\"hyprdots/01\"\n",
                Some(("hyprdots/01.jsonc", r#"{ "modules-left": ["clock"] }"#))
            );

            let modules = load_from(&dirs, &[]).expect("layout");

            assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Clock)]);
        }

        #[test]
        fn a_missing_install_answers_with_nothing() {
            let (_root, dirs) = install("", None);

            assert_eq!(load_from(&dirs, &[]), None);
        }
    }
}
