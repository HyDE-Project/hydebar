//! Resolving a named theme against the user's own theme files.
//!
//! The configuration names its palette with a single string —
//! `appearance = "catppuccin-mocha"` — and that string answers in two
//! places: a `themes` directory beside the configuration file first, the
//! themes shipped inside the binary second. A file shadows the shipped
//! theme of the same name, so experimenting never touches the binary.
//!
//! The substitution happens while the raw table is still in hand, next to
//! the module-layout presence check: whatever answers — file, builtin or
//! inline table — flows through the very same parse, validation and `HyDE`
//! overlay afterwards.

use std::{fs, path::Path};

use hydebar_proto::config::{preset_names, preset_source};
use serde::de::Error as _;

use super::ConfigReadError;

/// Subdirectory beside the configuration file holding user theme files.
pub(super) const THEMES_DIRECTORY: &str = "themes";

/// Replaces a named `appearance` with the theme it answers to.
///
/// An inline appearance table, or no appearance at all, passes through
/// untouched. A name answers in two places — a `themes` directory beside
/// the configuration file first, the themes shipped inside the binary
/// second — and a file shadows the shipped theme of the same name.
/// Anything else is a parse error naming every theme that would have
/// answered: the shipped ones and the files beside the configuration.
pub(super) fn resolve_named_theme(
    table: &mut toml::Table,
    config_path: &Path
) -> Result<(), ConfigReadError> {
    let Some(name) = table
        .get("appearance")
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
    else {
        return Ok(());
    };

    if !is_plain_name(&name) {
        return Err(invalid_name_error(config_path, &name));
    }

    let themes_dir = config_path
        .parent()
        .map(|parent| parent.join(THEMES_DIRECTORY));
    let base = base_table(&name, themes_dir.as_deref(), config_path)?;
    table.insert("appearance".to_owned(), toml::Value::Table(base));

    Ok(())
}

/// Reads the theme `name` answers to into a table.
///
/// Files beside the configuration win over the shipped themes; a base that
/// neither answers to is the same helpful error the loader reports for an
/// unknown name.
pub(super) fn base_table(
    name: &str,
    themes_dir: Option<&Path>,
    config_path: &Path
) -> Result<toml::Table, ConfigReadError> {
    if let Some(path) = themes_dir
        .map(|dir| dir.join(format!("{name}.toml")))
        .filter(|path| path.is_file())
    {
        let content = fs::read_to_string(&path).map_err(|source| ConfigReadError::Read {
            path: path.clone(),
            source
        })?;
        return toml::from_str(&content).map_err(|source| ConfigReadError::Parse {
            path,
            source
        });
    }

    if let Some(source) = preset_source(name) {
        return toml::from_str(source).map_err(|source| ConfigReadError::Parse {
            path: config_path.to_path_buf(),
            source
        });
    }

    Err(unknown_theme_error(config_path, name, themes_dir))
}

/// Whether `name` can name a file inside the themes directory.
///
/// Slashes and parent references would climb out of it, so they are refused
/// before any path is even built.
pub(super) fn is_plain_name(name: &str) -> bool {
    !(name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\'))
}

/// Builds the parse error for a theme name that climbs out of its directory.
pub(super) fn invalid_name_error(config_path: &Path, name: &str) -> ConfigReadError {
    ConfigReadError::Parse {
        path:   config_path.to_path_buf(),
        source: toml::de::Error::custom(format!(
            "invalid theme name '{name}': names stay within one directory"
        ))
    }
}

/// Builds the parse error for a name nothing answers to.
fn unknown_theme_error(
    config_path: &Path,
    name: &str,
    themes_dir: Option<&Path>
) -> ConfigReadError {
    let mut message = format!(
        "unknown theme '{name}'; shipped themes are: {}",
        preset_names().join(", ")
    );

    if let Some(dir) = themes_dir {
        let files = available_files(dir);
        if !files.is_empty() {
            message = format!(
                "{message}; theme files beside the config are: {}",
                files.join(", ")
            );
        }
    }

    ConfigReadError::Parse {
        path:   config_path.to_path_buf(),
        source: toml::de::Error::custom(message)
    }
}

/// Names the theme files beside the configuration, alphabetically.
fn available_files(themes_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(themes_dir) else {
        return Vec::new();
    };

    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .filter_map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_owned)
        })
        .collect();
    names.sort();

    names
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::path::PathBuf;

    use hex_color::HexColor;
    use hydebar_proto::{config::AppearanceColor, theme_source::HydeTheme};
    use tempfile::TempDir;

    use super::*;

    /// Joins the configuration directory with the file under test.
    fn config_in(root: &Path, appearance: &str) -> PathBuf {
        let path = root.join("config.toml");
        fs::write(&path, format!("appearance = \"{appearance}\"\n"))
            .expect("the test configuration writes");
        path
    }

    #[test]
    fn a_builtin_name_resolves_to_its_palette() {
        let root = TempDir::new().expect("temporary directory");
        let path = config_in(root.path(), "nord");
        let mut table: toml::Table =
            toml::from_str("appearance = \"nord\"").expect("the source parses");

        resolve_named_theme(&mut table, &path).expect("a builtin name resolves");

        let appearance = table
            .get("appearance")
            .and_then(toml::Value::as_table)
            .expect("the name stands in for its table");
        assert_eq!(
            appearance
                .get("background_color")
                .and_then(toml::Value::as_str),
            Some("#2e3440")
        );
    }

    #[test]
    fn a_file_shadows_the_builtin_of_the_same_name() {
        let root = TempDir::new().expect("temporary directory");
        let path = config_in(root.path(), "nord");
        fs::create_dir(root.path().join(THEMES_DIRECTORY)).expect("themes directory");
        fs::write(
            root.path().join(THEMES_DIRECTORY).join("nord.toml"),
            "background_color = \"#000000\"\n"
        )
        .expect("theme file writes");

        let mut table: toml::Table =
            toml::from_str("appearance = \"nord\"").expect("the source parses");
        resolve_named_theme(&mut table, &path).expect("the file resolves");

        let appearance = table.get("appearance").expect("the name is substituted");
        assert!(appearance.is_table(), "a file stands in for the name");
    }

    #[test]
    fn an_inline_appearance_table_passes_through_untouched() {
        let root = TempDir::new().expect("temporary directory");
        let path = config_in(root.path(), "nord");
        let mut table: toml::Table =
            toml::from_str("[appearance]\nopacity = 0.5\n").expect("the source parses");

        resolve_named_theme(&mut table, &path).expect("a table resolves");

        assert!(
            table
                .get("appearance")
                .and_then(toml::Value::as_table)
                .is_some()
        );
    }

    #[test]
    fn an_unknown_name_without_files_names_the_shipped_themes() {
        let root = TempDir::new().expect("temporary directory");
        let path = config_in(root.path(), "mocha");
        let mut table: toml::Table =
            toml::from_str("appearance = \"mocha\"").expect("the source parses");

        let err = resolve_named_theme(&mut table, &path).expect_err("unknown names fail");
        let message = err.to_string();

        assert!(message.contains("unknown theme 'mocha'"), "{message}");
        assert!(message.contains("catppuccin-mocha"), "{message}");
    }

    #[test]
    fn an_unknown_name_lists_the_files_beside_the_config() {
        let root = TempDir::new().expect("temporary directory");
        let path = config_in(root.path(), "mocha");
        fs::create_dir(root.path().join(THEMES_DIRECTORY)).expect("themes directory");
        fs::write(
            root.path().join(THEMES_DIRECTORY).join("my-theme.toml"),
            "background_color = \"#000000\"\n"
        )
        .expect("theme file writes");

        let mut table: toml::Table =
            toml::from_str("appearance = \"mocha\"").expect("the source parses");
        let err = resolve_named_theme(&mut table, &path).expect_err("unknown names fail");
        let message = err.to_string();

        assert!(message.contains("my-theme"), "{message}");
        assert!(message.contains("catppuccin-mocha"), "{message}");
    }

    #[test]
    fn a_broken_theme_file_reports_its_own_path() {
        let root = TempDir::new().expect("temporary directory");
        let path = config_in(root.path(), "broken");
        fs::create_dir(root.path().join(THEMES_DIRECTORY)).expect("themes directory");
        let theme_path = root.path().join(THEMES_DIRECTORY).join("broken.toml");
        fs::write(&theme_path, "opacity = [").expect("theme file writes");

        let mut table: toml::Table =
            toml::from_str("appearance = \"broken\"").expect("the source parses");
        let err = resolve_named_theme(&mut table, &path).expect_err("broken files fail");
        match err {
            ConfigReadError::Parse {
                path: reported, ..
            } => assert_eq!(reported, theme_path),
            ConfigReadError::Read {
                ..
            } => panic!("a broken theme file is a parse error, not a read error")
        }
    }

    #[test]
    fn a_name_climbing_out_of_the_directory_is_refused() {
        let root = TempDir::new().expect("temporary directory");
        let path = config_in(root.path(), "x");
        let mut table: toml::Table =
            toml::from_str("appearance = \"../outside\"").expect("the source parses");

        let err = resolve_named_theme(&mut table, &path).expect_err("climbing names fail");

        assert!(err.to_string().contains("invalid theme name"), "{err}");
    }

    #[test]
    fn a_theme_file_flows_into_the_loaded_configuration() {
        let root = TempDir::new().expect("temporary directory");
        let path = config_in(root.path(), "midnight");
        fs::create_dir(root.path().join(THEMES_DIRECTORY)).expect("themes directory");
        fs::write(
            root.path().join(THEMES_DIRECTORY).join("midnight.toml"),
            "background_color = \"#000000\"\n"
        )
        .expect("theme file writes");

        let config = super::super::read_config_with(&path, HydeTheme::default, |_| None)
            .expect("the configuration reads");

        assert_eq!(
            config.appearance.background_color,
            AppearanceColor::Simple(HexColor::rgb(0, 0, 0))
        );
    }
}
