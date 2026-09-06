//! Migrating a Waybar configuration to this bar's TOML.
//!
//! Waybar arranges its strip from three module arrays whose entries are
//! either module names or pill groups declared beside them — the same shape
//! `HyDE` layouts use, so the migration reuses [`module_for`] instead of
//! inventing a second mapping. Where a scripted custom has a native
//! counterpart the native module wins and the script is reported as
//! dropped; the rest rides along as custom modules with their commands.

use std::{collections::BTreeSet, fs, path::Path};

use hydebar_proto::bar_layout::{module_for, plain_json};
use serde_json::Value;

use super::{import_emit::render, import_scope::Scope, import_source};

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
    let groups = import_source::read_groups(input);
    let customs = import_source::read_customs(input);
    let empty = BTreeSet::new();
    let mut scope = Scope::new(
        &groups,
        customs
            .iter()
            .map(|custom| custom.name.as_str())
            .filter(|tail| module_for(&format!("custom/{tail}"), &empty).is_none())
            .collect(),
        customs.iter().map(|custom| custom.name.as_str()).collect()
    );

    let mut sections = Vec::new();
    for (section, key) in [
        ("left", "modules-left"),
        ("center", "modules-center"),
        ("right", "modules-right")
    ] {
        scope.begin_section();

        for member in input
            .get(key)
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice)
        {
            if let Some(name) = member.as_str() {
                scope.expand_entry(name);
            }
        }

        if scope.has_entries() {
            sections.push((section, scope.take_entries()));
        }
    }

    for custom in &customs {
        if !scope.is_encountered(&custom.name) {
            scope.note_unplaced(&custom.name);
        }
    }

    let placed = scope.take_placed();
    let (unmapped, dropped_scripts) = scope.take_reports();
    Converted {
        position: import_source::read_edge(input.get("position")),
        layer: import_source::read_layer(input.get("layer")),
        sections,
        clock_format: input
            .get("clock")
            .and_then(Value::as_object)
            .and_then(|clock| clock.get("format"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        customs: customs
            .into_iter()
            .filter(|custom| placed.contains(&custom.name))
            .collect(),
        unmapped,
        dropped_scripts
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
        assert_eq!(converted.unmapped, vec!["mystery".to_owned()]);
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
