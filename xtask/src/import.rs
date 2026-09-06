//! Migrating a Waybar configuration to this bar's TOML.
//!
//! Waybar arranges its strip from three module arrays whose entries are
//! either module names or pill groups declared beside them — the same shape
//! `HyDE` layouts use, so the migration reuses [`module_for`] instead of
//! inventing a second mapping. Where a scripted custom has a native
//! counterpart the native module wins and the script is reported as
//! dropped; the rest rides along as custom modules with their commands.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path
};

use hydebar_proto::{
    bar_layout::{module_for, plain_json},
    config::ModuleName
};
use serde_json::Value;

use super::import_emit::render;

/// One bar section after migration.
pub enum SectionEntry {
    /// One module, standing on its own island.
    Single(String),
    /// A Waybar pill group, kept as one island.
    Group(Vec<String>)
}

/// A Waybar custom module carried over with its commands.
pub struct CustomEntry {
    /// Module key without the `custom/` prefix.
    pub name:     String,
    /// Command producing the readout.
    pub exec:     Option<String>,
    /// Seconds between runs, if the definition schedules any.
    pub interval: Option<u64>,
    /// Signal offset refreshing on demand, if the definition names one.
    pub signal:   Option<u8>,
    /// Click command, if the definition answers presses.
    pub command:  Option<String>
}

/// Everything a Waybar configuration becomes.
pub struct Converted {
    /// Bar edge, when the source names one this bar knows.
    pub position:        Option<String>,
    /// Compositor layer, when the source names one this bar knows.
    pub layer:           Option<String>,
    /// Sections in bar order, skipping the empty ones.
    pub sections:        Vec<(&'static str, Vec<SectionEntry>)>,
    /// Clock format string, when the source states one.
    pub clock_format:    Option<String>,
    /// Carried-over custom modules, in definition order.
    pub customs:         Vec<CustomEntry>,
    /// Entries with no counterpart, in first-seen order.
    pub unmapped:        Vec<String>,
    /// Scripted customs a native module displaced.
    pub dropped_scripts: Vec<String>
}

/// Reads a Waybar `config.jsonc`, comments and trailing commas included.
///
/// # Errors
///
/// Returns a message when the file cannot be read or parsed.
pub fn load(path: &Path) -> Result<Value, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&plain_json(&content))
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

/// Converts a parsed Waybar configuration.
pub fn convert(input: &Value) -> Converted {
    let groups = read_groups(input);
    let customs = read_customs(input);
    let empty = BTreeSet::new();
    let custom_set: BTreeSet<&str> = customs
        .iter()
        .map(|(_, tail)| tail.as_str())
        .filter(|tail| module_for(&format!("custom/{tail}"), &empty).is_none())
        .collect();

    let mut sections = Vec::new();
    let mut unmapped = Vec::new();
    let mut dropped_scripts = Vec::new();
    let mut placed_customs: BTreeSet<String> = BTreeSet::new();

    for (section, key) in [
        ("left", "modules-left"),
        ("center", "modules-center"),
        ("right", "modules-right")
    ] {
        let mut entries = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut visiting = Vec::new();

        for member in input
            .get(key)
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice)
        {
            if let Some(name) = member.as_str() {
                expand_entry(
                    name,
                    &groups,
                    &custom_set,
                    &mut visiting,
                    &mut entries,
                    &mut seen,
                    &mut unmapped,
                    &mut dropped_scripts,
                    &mut placed_customs,
                    &customs
                );
            }
        }

        if !entries.is_empty() {
            sections.push((section, entries));
        }
    }

    for (_, tail) in &customs {
        if !placed_customs.contains(tail) && !unmapped.iter().any(|name| name == tail) {
            unmapped.push(format!("custom/{tail} (defined but not placed)"));
        }
    }

    Converted {
        position: read_edge(input.get("position")),
        layer: read_layer(input.get("layer")),
        sections,
        clock_format: input
            .get("clock")
            .and_then(Value::as_object)
            .and_then(|clock| clock.get("format"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        customs: customs
            .into_iter()
            .filter(|(_, tail)| placed_customs.contains(tail))
            .map(|(entry, _)| entry)
            .collect(),
        unmapped,
        dropped_scripts
    }
}

/// Pill groups declared beside the module arrays, keyed by their full tail.
///
/// `pill#1` and `pill#2` are distinct groups that happen to share a family;
/// the family name is only a fallback when nothing answers to the exact
/// tail, so collapsing them here would silently rewire the strip.
fn read_groups(input: &Value) -> BTreeMap<String, Vec<String>> {
    let mut groups = BTreeMap::new();

    if let Some(object) = input.as_object() {
        for (key, value) in object {
            if let Some(group) = key.strip_prefix("group/") {
                let members: Vec<String> = value
                    .get("modules")
                    .and_then(Value::as_array)
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                groups.insert(group.to_owned(), members);
            }
        }
    }

    groups
}

/// Scripted customs: tails carrying their definitions.
fn read_customs(input: &Value) -> Vec<(CustomEntry, String)> {
    let mut customs = Vec::new();

    if let Some(object) = input.as_object() {
        for (key, value) in object {
            if let Some(tail) = key.strip_prefix("custom/") {
                let exec = value.get("exec").and_then(Value::as_str).map(str::to_owned);
                if exec.is_none() {
                    continue;
                }
                customs.push((
                    CustomEntry {
                        name: tail.to_owned(),
                        exec,
                        interval: value.get("interval").and_then(Value::as_u64),
                        signal: value
                            .get("signal")
                            .and_then(Value::as_u64)
                            .and_then(|signal| u8::try_from(signal).ok()),
                        command: value
                            .get("on-click")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    },
                    tail.to_owned()
                ));
            }
        }
    }

    customs
}

/// Expands one section entry, following pill groups recursively.
///
/// A group becomes one island of whatever its members map to; cycles fall
/// away silently, and whatever maps to nothing is reported, never emitted.
#[allow(clippy::too_many_arguments)]
fn expand_entry(
    name: &str,
    groups: &BTreeMap<String, Vec<String>>,
    custom_set: &BTreeSet<&str>,
    visiting: &mut Vec<String>,
    entries: &mut Vec<SectionEntry>,
    seen: &mut BTreeSet<String>,
    unmapped: &mut Vec<String>,
    dropped_scripts: &mut Vec<String>,
    placed_customs: &mut BTreeSet<String>,
    customs: &[(CustomEntry, String)]
) {
    if let Some(group) = name.strip_prefix("group/") {
        if visiting.iter().any(|known| known == group) {
            return;
        }
        visiting.push(group.to_owned());

        // The exact tail first (`pill#1`); the un-suffixed family only when
        // nothing answers to it.
        let base = group.split('#').next().unwrap_or(group);
        let declared = groups
            .get(group)
            .or_else(|| {
                if base == group {
                    None
                } else {
                    groups.get(base)
                }
            })
            .cloned()
            .unwrap_or_default();

        let mut members = Vec::new();
        for member in &declared {
            map_member(
                member,
                custom_set,
                &mut members,
                seen,
                unmapped,
                dropped_scripts,
                placed_customs,
                customs
            );
        }
        visiting.pop();

        if !members.is_empty() {
            entries.push(SectionEntry::Group(members));
        }
        return;
    }

    let mut single = Vec::new();
    map_member(
        name,
        custom_set,
        &mut single,
        seen,
        unmapped,
        dropped_scripts,
        placed_customs,
        customs
    );
    entries.extend(single.into_iter().map(SectionEntry::Single));
}

/// Maps one Waybar module name, tracking scripts and the unknown.
#[allow(clippy::too_many_arguments)]
fn map_member(
    name: &str,
    custom_set: &BTreeSet<&str>,
    members: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    unmapped: &mut Vec<String>,
    dropped_scripts: &mut Vec<String>,
    placed_customs: &mut BTreeSet<String>,
    customs: &[(CustomEntry, String)]
) {
    let plain = name.split('#').next().unwrap_or(name);

    match module_for(plain, custom_set) {
        None => {
            if !unmapped.iter().any(|known| known == name) {
                unmapped.push(name.to_owned());
            }
        }
        Some(ModuleName::Custom(tail)) => {
            if seen.insert(tail.clone()) {
                placed_customs.insert(tail.clone());
                members.push(tail);
            }
        }
        Some(module) => {
            if customs
                .iter()
                .any(|(_, tail)| plain == format!("custom/{tail}"))
                && !dropped_scripts.iter().any(|known| known == name)
            {
                dropped_scripts.push(name.to_owned());
            }
            let spelling = module.as_str().to_owned();
            if seen.insert(spelling.clone()) {
                members.push(spelling);
            }
        }
    }
}

/// Bar edge the Waybar position names, if this bar knows it.
fn read_edge(value: Option<&Value>) -> Option<String> {
    match value.and_then(Value::as_str) {
        Some("top") => Some("Top".to_owned()),
        Some("bottom") => Some("Bottom".to_owned()),
        _ => None
    }
}

/// Compositor layer the Waybar layer names, if this bar knows it.
fn read_layer(value: Option<&Value>) -> Option<String> {
    match value.and_then(Value::as_str) {
        Some("background") => Some("Background".to_owned()),
        Some("bottom") => Some("Bottom".to_owned()),
        Some("top") => Some("Top".to_owned()),
        Some("overlay") => Some("Overlay".to_owned()),
        _ => None
    }
}

/// Runs the `import-waybar` command: `import-waybar <config.jsonc> [--out
/// <config.toml>]`.
///
/// Without `--out` the migration prints to standard output; with it, the
/// file is written unless it already exists. Reports go to standard error
/// either way.
///
/// # Errors
///
/// Returns a message when the arguments, the source or the destination are
/// unusable.
pub fn run_command(arguments: &[String]) -> Result<(), String> {
    let mut source = None;
    let mut out = None;
    let mut rest = arguments.iter();

    while let Some(argument) = rest.next() {
        if argument == "--out" {
            out = Some(
                rest.next()
                    .map(String::as_str)
                    .ok_or_else(|| "--out needs a value".to_owned())?
            );
        } else if argument.starts_with('-') {
            return Err(format!("unknown flag '{argument}'"));
        } else if source.is_some() {
            return Err("one source file, got more".to_owned());
        } else {
            source = Some(argument.as_str());
        }
    }

    let source = source.ok_or_else(|| "a waybar configuration is required".to_owned())?;
    let converted = convert(&load(Path::new(source))?);
    let rendered = render(&converted, source);

    if let Some(path) = out {
        if Path::new(path).exists() {
            return Err(format!("refusing to overwrite {path}"));
        }
        std::fs::write(path, &rendered)
            .map_err(|error| format!("cannot write {path}: {error}"))?;
        println!("wrote {path}");
    } else {
        print!("{rendered}");
    }

    for name in &converted.unmapped {
        eprintln!("xtask: no counterpart for {name}");
    }
    for name in &converted.dropped_scripts {
        eprintln!("xtask: native module used, script dropped: {name}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn converted_fixture() -> Value {
        serde_json::from_str(&plain_json(
            r#"{
            // a comment the strict parser would choke on,
            "modules-left": ["group/leaf",],
            "modules-center": ["clock", "custom/updates", "custom/gamemode", "mystery"],
            "modules-right": ["battery", "battery", "custom/weather", "custom/mine"],
            "group/leaf": {
                "modules": ["hyprland/workspaces", "custom/mine"]
            },
            "clock": { "format": "%H:%M" },
            "layer": "top",
            "position": "top",
            "custom/updates": { "exec": "checkupdates" },
            "custom/weather": { "exec": "weather.sh" },
            "custom/mine": { "exec": "mine.sh", "interval": 30, "signal": 7, "on-click": "mine-open" }
        }"#
        ))
        .expect("the fixture parses")
    }

    #[test]
    fn groups_expand_and_duplicates_fall_away() {
        let converted = convert(&converted_fixture());

        assert_eq!(converted.sections.len(), 3);
        let left = &converted.sections[0].1;
        assert_eq!(left.len(), 1);
        match &left[0] {
            SectionEntry::Group(members) => {
                assert!(members.contains(&"Workspaces".to_owned()));
                assert!(members.contains(&"mine".to_owned()));
            }
            SectionEntry::Single(_) => panic!("the leaf stays a group")
        }

        let right = &converted.sections[2].1;
        assert_eq!(
            right
                .iter()
                .filter(|entry| matches!(entry, SectionEntry::Single(name) if name == "Battery"))
                .count(),
            1
        );
    }

    #[test]
    fn native_aliases_win_and_scripts_are_reported() {
        let converted = convert(&converted_fixture());

        assert!(
            converted
                .dropped_scripts
                .contains(&"custom/weather".to_owned())
        );
        assert!(converted.unmapped.contains(&"mystery".to_owned()));
        assert_eq!(converted.customs.len(), 1);
        assert_eq!(converted.customs[0].name, "mine");
        assert_eq!(converted.customs[0].interval, Some(30));
        assert_eq!(converted.customs[0].signal, Some(7));
        assert_eq!(converted.customs[0].command, Some("mine-open".to_owned()));
    }

    #[test]
    fn suffixed_groups_stay_distinct() {
        let input: Value = serde_json::from_str(
            r#"{
            "modules-center": ["group/pill#1", "group/pill#2", "group/pill"],
            "group/pill#1": { "modules": ["clock"] },
            "group/pill#2": { "modules": ["battery"] }
        }"#
        )
        .expect("the fixture parses");

        let converted = convert(&input);

        assert_eq!(converted.sections.len(), 1);
        let rendered: Vec<String> = converted.sections[0]
            .1
            .iter()
            .map(|entry| match entry {
                SectionEntry::Single(name) => name.clone(),
                SectionEntry::Group(members) => members.join("+")
            })
            .collect();

        assert_eq!(rendered, vec!["Clock", "Battery"]);
    }

    #[test]
    fn bar_chrome_reads_across() {
        let converted = convert(&converted_fixture());

        assert_eq!(converted.position, Some("Top".to_owned()));
        assert_eq!(converted.layer, Some("Top".to_owned()));
        assert_eq!(converted.clock_format, Some("%H:%M".to_owned()));
    }

    #[test]
    fn the_rendered_migration_parses_as_a_configuration() {
        use hydebar_proto::config::{Config, ModuleName};

        let converted = convert(&converted_fixture());
        let rendered = super::super::import_emit::render(&converted, "fixture");

        let config: Config = toml::from_str(&rendered).expect("the migration output parses");
        let placed: Vec<&ModuleName> = config.modules.placed().collect();

        assert!(placed.contains(&&ModuleName::Clock));
        assert!(placed.contains(&&ModuleName::Battery));
        assert!(placed.contains(&&ModuleName::Workspaces));
        assert_eq!(config.clock.format, "%H:%M");
        assert_eq!(config.custom_modules.len(), 1);
        assert_eq!(config.custom_modules[0].name, "mine");
    }
}
