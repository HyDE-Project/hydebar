//! Saying out loud which settings the file names and the bar does not know.
//!
//! A misspelled key is the quietest way a configuration can fail: serde skips
//! what it does not recognise, the bar starts with the default in force, and
//! nothing anywhere says that `font_sze` was read and thrown away. Every key
//! the deserializer walks past is collected here and written to the log with
//! the file it came from, so a setting that does nothing says so once at
//! startup instead of being hunted for.

use std::{cell::RefCell, path::Path};

use hydebar_proto::config::{Appearance, Config};
use log::warn;

/// Reads a configuration out of a parsed file, naming every key it does not
/// know.
///
/// The keys are reported as their full path — `weather.locaton` rather than
/// `locaton` — because the same leaf can be misspelled under any number of
/// sections.
///
/// # Errors
///
/// Returns the parse error when the file cannot be read as a configuration at
/// all. A key that is merely unknown is never an error: the bar starts with
/// the default in force, and the log says which key did nothing.
pub(super) fn read_naming_unknown(
    table: toml::Table,
    path: &Path
) -> Result<Config, toml::de::Error> {
    let inside_appearance = appearance_unknown(&table);
    let unknown = RefCell::new(inside_appearance);

    let config = serde_ignored::deserialize(table, |key| {
        unknown.borrow_mut().push(key.to_string());
    })?;

    report(&unknown.into_inner(), path);

    Ok(config)
}

/// Names the unknown keys inside the appearance section.
///
/// That section is read through a deserializer accepting either a preset name
/// or a whole table, and an untagged choice buffers what it is handed — so the
/// walk above passes over the section without ever seeing inside it. The table
/// is therefore read a second time, on its own, and its keys are reported at
/// the path they live at.
///
/// A section that is a preset name, or one the bar cannot read at all, yields
/// nothing: the first is not a table to walk and the second is a parse error
/// the caller is about to raise anyway.
fn appearance_unknown(table: &toml::Table) -> Vec<String> {
    let Some(toml::Value::Table(appearance)) = table.get("appearance") else {
        return Vec::new();
    };

    let unknown = RefCell::new(Vec::new());
    let read: Result<Appearance, _> = serde_ignored::deserialize(appearance.clone(), |key| {
        unknown.borrow_mut().push(format!("appearance.{key}"));
    });

    read.map_or_else(|_| Vec::new(), |_: Appearance| unknown.into_inner())
}

/// Writes one line per unknown key, or nothing when there are none.
fn report(unknown: &[String], path: &Path) {
    for key in unknown {
        warn!(
            "{}: `{key}` is not a setting the bar knows; it was read and ignored",
            path.display()
        );
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{cell::RefCell, path::Path};

    use super::{appearance_unknown, read_naming_unknown};

    fn unknown_keys(source: &str) -> Vec<String> {
        let table: toml::Table = toml::from_str(source).expect("the source parses as TOML");
        let mut keys = appearance_unknown(&table);
        let unknown = RefCell::new(Vec::new());

        let _: hydebar_proto::config::Config = serde_ignored::deserialize(table, |key| {
            unknown.borrow_mut().push(key.to_string());
        })
        .expect("the source reads as a configuration");

        keys.extend(unknown.into_inner());
        keys.sort();

        keys
    }

    #[test]
    fn a_file_naming_only_known_settings_reports_nothing() {
        assert!(unknown_keys("log_level = \"warn\"\n").is_empty());
    }

    #[test]
    fn a_misspelled_top_level_setting_is_named() {
        assert_eq!(unknown_keys("log_levle = \"warn\"\n"), vec!["log_levle"]);
    }

    #[test]
    fn a_misspelled_setting_is_named_by_its_whole_path() {
        assert_eq!(
            unknown_keys("[weather]\nlocaton = \"here\"\n"),
            vec!["weather.locaton"]
        );
    }

    #[test]
    fn a_section_the_bar_never_heard_of_is_named() {
        assert_eq!(
            unknown_keys("[weahter]\nlocation = \"here\"\n"),
            vec!["weahter"]
        );
    }

    #[test]
    fn every_unknown_key_is_named_rather_than_the_first() {
        assert_eq!(
            unknown_keys("log_levle = \"warn\"\nposiiton = \"top\"\n"),
            vec!["log_levle", "posiiton"]
        );
    }

    #[test]
    fn a_misspelled_appearance_setting_is_named_despite_the_preset_reader() {
        assert_eq!(
            unknown_keys("[appearance]\nopacty = 0.5\n"),
            vec!["appearance.opacty"]
        );
    }

    #[test]
    fn a_misspelled_setting_under_the_appearance_menu_is_named() {
        assert_eq!(
            unknown_keys("[appearance.menu]\nopacty = 0.5\n"),
            vec!["appearance.menu.opacty"]
        );
    }

    #[test]
    fn an_appearance_named_as_a_preset_reports_nothing() {
        assert!(unknown_keys("appearance = \"catppuccin-mocha\"\n").is_empty());
    }

    #[test]
    fn a_file_that_reads_at_all_still_reads_when_a_key_is_unknown() {
        let table: toml::Table =
            toml::from_str("log_level = \"warn\"\nlog_levle = \"trace\"\n").expect("TOML");

        let config = read_naming_unknown(table, Path::new("/tmp/config.toml"))
            .expect("the configuration reads");

        assert_eq!(config.log_level, "warn");
    }
}
