//! Restating a layout source in the bar's own modules.

use std::collections::BTreeSet;

use serde_json::Value;

use super::{
    RestatedLayout, jsonc::plain_json, mapping::module_for, synth::synthesize_text_entry
};
use crate::config::{CustomModuleDef, ModuleDef, ModuleName, Modules};

/// Position arrays a layout may carry, in the order the bar draws them.
const SECTIONS: [&str; 3] = ["modules-left", "modules-center", "modules-right"];

/// Restates a layout source in the bar's own modules.
///
/// Returns [`None`] for a source that does not parse or maps to nothing at
/// all: an empty bar helps nobody, and the default layout is the better
/// answer to a layout the bar cannot restate.
#[must_use]
pub fn parse(source: &str, custom_names: &[String]) -> Option<RestatedLayout> {
    let root: Value = serde_json::from_str(&plain_json(source)).ok()?;
    let custom: BTreeSet<&str> = custom_names.iter().map(String::as_str).collect();
    let mut placed = BTreeSet::new();
    let mut synthesized = Vec::new();

    let mut sections = SECTIONS
        .iter()
        .map(|section| section_defs(&root, section, &custom, &mut placed, &mut synthesized));

    let modules = Modules {
        left:   sections.next()?,
        center: sections.next()?,
        right:  sections.next()?
    };

    (modules.placed().count() > 0).then_some(RestatedLayout {
        modules,
        synthesized
    })
}

/// Restates one position array of the layout.
fn section_defs(
    root: &Value,
    section: &str,
    custom: &BTreeSet<&str>,
    placed: &mut BTreeSet<ModuleName>,
    synthesized: &mut Vec<CustomModuleDef>
) -> Vec<ModuleDef> {
    let Some(entries) = root.get(section).and_then(Value::as_array) else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|entry| entry_def(root, entry, custom, placed, synthesized))
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
    placed: &mut BTreeSet<ModuleName>,
    synthesized: &mut Vec<CustomModuleDef>
) -> Option<ModuleDef> {
    if entry.starts_with("group/") {
        let members: Vec<ModuleName> = root
            .get(entry)
            .and_then(|group| group.get("modules"))
            .and_then(Value::as_array)?
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|member| place(root, member, custom, placed, synthesized))
            .collect();

        return match members.len() {
            0 => None,
            1 => Some(ModuleDef::Single(members.into_iter().next()?)),
            _ => Some(ModuleDef::Group(members))
        };
    }

    place(root, entry, custom, placed, synthesized).map(ModuleDef::Single)
}

/// Maps one layout entry onto a module of ours, at most once per layout.
///
/// Several layout entries can land on the same module — the speaker and
/// microphone are one audio module here — and placing it twice would draw it
/// twice. The first entry wins the spot; later ones fall away.
fn place(
    root: &Value,
    name: &str,
    custom: &BTreeSet<&str>,
    placed: &mut BTreeSet<ModuleName>,
    synthesized: &mut Vec<CustomModuleDef>
) -> Option<ModuleName> {
    let Some(module) = module_for(name, custom).or_else(|| {
        synthesize_text_entry(root, name, custom).map(|definition| {
            let module = ModuleName::Custom(definition.name.clone());
            synthesized.push(definition);
            module
        })
    }) else {
        log::warn!("bar layout entry `{name}` has no counterpart here and is skipped");

        return None;
    };

    placed.insert(module.clone()).then_some(module)
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
        let modules = parse(LAYOUT, &custom).expect("layout").modules;

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
        let modules = parse(LAYOUT, &[]).expect("layout").modules;
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
        let custom = vec!["my_widget".to_owned()];
        let source = r#"{ "modules-left": ["custom/my_widget", "custom/updates"] }"#;

        let modules = parse(source, &custom).expect("layout").modules;

        assert_eq!(
            modules.left,
            vec![
                ModuleDef::Single(ModuleName::Custom("my_widget".to_owned())),
                ModuleDef::Single(ModuleName::Updates),
            ]
        );
    }

    /// The wallpaper and layout seats belong to the built-ins even when a
    /// legacy wrapper of the same name is configured: the wrappers speak
    /// one command, the built-ins speak the whole mouse.
    #[test]
    fn the_wallpaper_and_layout_seats_always_go_to_the_built_ins() {
        let custom = vec!["wallchange".to_owned(), "wbar".to_owned()];
        let source = r#"{ "modules-left": ["custom/wallchange", "custom/wbar"] }"#;

        let modules = parse(source, &custom).expect("layout").modules;

        assert_eq!(
            modules.left,
            vec![
                ModuleDef::Single(ModuleName::Wallpaper),
                ModuleDef::Single(ModuleName::BarLayout),
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

        let modules = parse(source, &custom).expect("layout").modules;

        assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Themes)]);
    }

    #[test]
    fn a_bare_module_list_needs_no_groups() {
        let source = r#"{
            "modules-left": ["custom/launcher", "hyprland/window"],
            "modules-right": ["clock"]
        }"#;

        let modules = parse(source, &[]).expect("layout").modules;

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
            "group/pill#a": { "modules": ["no/counterpart", "custom/no-counterpart"] },
            "group/pill#b": { "modules": ["no/counterpart", "clock"] }
        }"#;

        let modules = parse(source, &[]).expect("layout").modules;

        assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Clock)]);
    }

    /// A layout mapping to nothing at all is refused so the caller keeps a
    /// layout that draws something.
    #[test]
    fn a_layout_the_bar_cannot_restate_is_refused() {
        assert_eq!(
            parse(r#"{ "modules-left": ["no/counterpart"] }"#, &[]),
            None
        );
        assert_eq!(parse("not json at all", &[]), None);
    }

    #[test]
    fn variant_suffixes_pick_the_same_module() {
        let source = r#"{ "modules-left": ["clock#date", "network#bandwidthUpBytes"] }"#;

        let modules = parse(source, &[]).expect("layout").modules;

        assert_eq!(
            modules.left,
            vec![
                ModuleDef::Single(ModuleName::Clock),
                ModuleDef::Single(ModuleName::Network),
            ]
        );
    }
}
