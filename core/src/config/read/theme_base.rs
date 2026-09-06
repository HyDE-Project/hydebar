//! Layering an `[appearance]` table over a named base theme.
//!
//! A configuration that only tweaks a shipped or filed theme says so with
//! one key — `theme = "catppuccin-mocha"` beside its overrides — instead of
//! copying the whole palette. The base is read the same way a named
//! `appearance` is, file first and shipped second, and the overrides win
//! key by key, the way the user's own files always win over the desktop's
//! theme. Nested tables merge recursively, so overriding one menu value
//! keeps the rest of the menu it came with.
//!
//! A base file carrying its own `theme` key is not followed further: one
//! layer is the whole feature, and the leftover key is reported as unknown
//! like any other misspelling.

use std::path::Path;

use super::{
    ConfigReadError,
    theme::{base_table, invalid_name_error, is_plain_name}
};
use crate::config::read::theme::THEMES_DIRECTORY;

/// Key inside `[appearance]` naming the theme to layer under the overrides.
const THEME_BASE_KEY: &str = "theme";

/// Merges an `[appearance]` table over the base theme it names.
///
/// A missing section, a section without the key, or a section naming
/// nothing passes through untouched. The consumed `theme` key is removed,
/// so the unknown-key walk afterwards never reports the machinery itself.
pub(super) fn resolve_theme_base(
    table: &mut toml::Table,
    config_path: &Path
) -> Result<(), ConfigReadError> {
    let Some(appearance) = table.get_mut("appearance") else {
        return Ok(());
    };
    let Some(appearance) = appearance.as_table_mut() else {
        return Ok(());
    };
    let Some(base) = appearance
        .get(THEME_BASE_KEY)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
    else {
        return Ok(());
    };

    if !is_plain_name(&base) {
        return Err(invalid_name_error(config_path, &base));
    }

    let themes_dir = config_path
        .parent()
        .map(|parent| parent.join(THEMES_DIRECTORY));
    let mut merged = base_table(&base, themes_dir.as_deref(), config_path)?;

    let mut overrides = std::mem::take(appearance);
    overrides.remove(THEME_BASE_KEY);
    merge_tables(&mut merged, overrides);
    *appearance = merged;

    Ok(())
}

/// Folds the overriding table into the base one, recursing into nested
/// tables and replacing anything else — scalars and arrays name their
/// value whole, so an override never concatenates with what it replaces.
fn merge_tables(base: &mut toml::Table, over: toml::Table) {
    for (key, value) in over {
        let merged = match (base.remove(&key), value) {
            (Some(toml::Value::Table(mut inner)), toml::Value::Table(over_inner)) => {
                merge_tables(&mut inner, over_inner);
                toml::Value::Table(inner)
            }
            (_, value) => value
        };
        base.insert(key, merged);
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]
    use std::{fs, path::PathBuf};

    use hydebar_proto::{config::Appearance, theme_source::HydeTheme};
    use tempfile::TempDir;

    use super::*;
    use crate::config::{read::theme::THEMES_DIRECTORY, read_config_with};

    fn config_path(root: &Path) -> PathBuf {
        root.join("config.toml")
    }

    fn table(source: &str) -> toml::Table {
        toml::from_str(source).expect("the source parses as TOML")
    }

    #[test]
    fn a_section_without_a_theme_key_passes_through_untouched() {
        let root = TempDir::new().expect("temporary directory");
        let mut table = table("[appearance]\nopacity = 0.5\n");

        resolve_theme_base(&mut table, &config_path(root.path()))
            .expect("a keyless table resolves");

        let appearance = table
            .get("appearance")
            .and_then(toml::Value::as_table)
            .expect("the section stays a table");
        assert_eq!(
            appearance.get("opacity").and_then(toml::Value::as_float),
            Some(0.5)
        );
    }

    #[test]
    fn a_named_appearance_string_is_left_for_the_other_resolver() {
        let root = TempDir::new().expect("temporary directory");
        let mut table = table("appearance = \"nord\"\n");

        resolve_theme_base(&mut table, &config_path(root.path()))
            .expect("a string resolves elsewhere");

        assert_eq!(
            table.get("appearance").and_then(toml::Value::as_str),
            Some("nord")
        );
    }

    #[test]
    fn overrides_win_over_the_named_base_key_by_key() {
        let root = TempDir::new().expect("temporary directory");
        let mut table = table(
            "[appearance]\ntheme = \"nord\"\nopacity = 0.5\nbackground_color = \"#000000\"\n"
        );

        resolve_theme_base(&mut table, &config_path(root.path())).expect("the base merges");

        let appearance = table
            .get("appearance")
            .and_then(toml::Value::as_table)
            .expect("the section stays a table");
        assert!(
            appearance.get(THEME_BASE_KEY).is_none(),
            "the key is consumed"
        );
        assert_eq!(
            appearance.get("opacity").and_then(toml::Value::as_float),
            Some(0.5)
        );
        assert_eq!(
            appearance
                .get("background_color")
                .and_then(toml::Value::as_str),
            Some("#000000")
        );
        assert!(
            appearance
                .get("primary_color")
                .and_then(toml::Value::as_str)
                .is_some(),
            "untouched keys ride in on the base"
        );
    }

    #[test]
    fn nested_tables_merge_recursively() {
        let root = TempDir::new().expect("temporary directory");
        let mut table =
            table("[appearance]\ntheme = \"nord\"\n[appearance.menu]\nopacity = 0.5\n");

        resolve_theme_base(&mut table, &config_path(root.path())).expect("the base merges");

        let menu = table
            .get("appearance")
            .and_then(toml::Value::as_table)
            .and_then(|appearance| appearance.get("menu"))
            .and_then(toml::Value::as_table)
            .expect("the menu section survives");
        assert_eq!(
            menu.get("opacity").and_then(toml::Value::as_float),
            Some(0.5)
        );
        assert!(
            menu.get("backdrop")
                .and_then(toml::Value::as_float)
                .is_some(),
            "the rest of the menu rides in on the base"
        );
    }

    #[test]
    fn a_user_file_answers_as_a_base() {
        let root = TempDir::new().expect("temporary directory");
        fs::create_dir(root.path().join(THEMES_DIRECTORY)).expect("themes directory");
        fs::write(
            root.path().join(THEMES_DIRECTORY).join("midnight.toml"),
            "background_color = \"#000000\"\n"
        )
        .expect("theme file writes");

        let mut table = table("[appearance]\ntheme = \"midnight\"\nopacity = 0.5\n");
        resolve_theme_base(&mut table, &config_path(root.path())).expect("the file merges");

        let appearance = table
            .get("appearance")
            .and_then(toml::Value::as_table)
            .expect("the section stays a table");
        assert_eq!(
            appearance
                .get("background_color")
                .and_then(toml::Value::as_str),
            Some("#000000")
        );
        assert_eq!(
            appearance.get("opacity").and_then(toml::Value::as_float),
            Some(0.5)
        );
    }

    #[test]
    fn an_unknown_base_name_says_what_would_have_answered() {
        let root = TempDir::new().expect("temporary directory");
        let mut table = table("[appearance]\ntheme = \"mocha\"\n");

        let err =
            resolve_theme_base(&mut table, &config_path(root.path())).expect_err("unknown fails");
        let message = err.to_string();

        assert!(message.contains("unknown theme 'mocha'"), "{message}");
        assert!(message.contains("catppuccin-mocha"), "{message}");
    }

    #[test]
    fn a_base_name_climbing_out_is_refused() {
        let root = TempDir::new().expect("temporary directory");
        let mut table = table("[appearance]\ntheme = \"../outside\"\n");

        let err =
            resolve_theme_base(&mut table, &config_path(root.path())).expect_err("climbing fails");

        assert!(err.to_string().contains("invalid theme name"), "{err}");
    }

    #[test]
    fn a_base_layers_into_the_loaded_configuration() {
        let root = TempDir::new().expect("temporary directory");
        let path = root.path().join("config.toml");
        fs::write(&path, "[appearance]\ntheme = \"nord\"\nopacity = 0.5\n")
            .expect("the test configuration writes");

        let config = read_config_with(&path, HydeTheme::default, |_| None)
            .expect("the configuration reads");

        assert_eq!(config.appearance.opacity, 0.5);
        assert_ne!(
            config.appearance.background_color,
            Appearance::default().background_color,
            "the palette rides in on the base"
        );
    }
}
