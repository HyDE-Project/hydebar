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
//!
//! The restatement itself lives in [`parse`], the entry-to-module tables in
//! [`mapping`], the invented text entries in [`synth`], the human spelling of
//! configured names in [`labels`] and the relaxed-JSON stripping in [`jsonc`].
//! This file keeps the restated layout type and the walk from `staterc` to
//! the layout file it records.

mod jsonc;
mod labels;
mod mapping;
mod parse;
mod synth;

use std::{fs, path::PathBuf};

pub use jsonc::plain_json;
pub use labels::display_label;
use log::{debug, warn};
pub use parse::parse;

use crate::{
    config::{CustomModuleDef, Modules},
    hyde_dirs::HydeDirs,
    shell_vars
};

/// A layout restated in the bar's modules, with the entries it had to invent.
///
/// The decorative layouts carry static text entries defined inline in the
/// layout file — a label and possibly a click command, nothing else. They map
/// to no built-in, so the restatement synthesises a custom module for each:
/// the label becomes the glyph, the click command stays the click command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestatedLayout {
    /// The three position arrays in the bar's own modules.
    pub modules:     Modules,
    /// Custom modules invented for inline text entries, ready to adopt.
    pub synthesized: Vec<CustomModuleDef>
}

/// Key under which `staterc` records the layout file in force.
const LAYOUT_PATH_KEY: &str = "WAYBAR_LAYOUT_PATH";

/// Key under which `staterc` records the layout's name.
const LAYOUT_NAME_KEY: &str = "WAYBAR_LAYOUT_NAME";

/// Reads the active layout from the user's own directories.
///
/// `custom_names` are the custom modules the bar configuration defines; a
/// layout entry naming one of them is placed as that module, so a user's own
/// wrapper wins over the built-in the name would otherwise map to.
#[must_use]
pub fn load(custom_names: &[String]) -> Option<RestatedLayout> {
    HydeDirs::from_env().and_then(|dirs| load_from(&dirs, custom_names))
}

/// Reads the active layout from an explicit `HyDE` install.
///
/// Returns [`None`] whenever any link of the chain is missing — no `staterc`,
/// no readable layout, or a layout none of whose entries the bar can place —
/// and the caller keeps the layout it already has.
#[must_use]
pub fn load_from(dirs: &HydeDirs, custom_names: &[String]) -> Option<RestatedLayout> {
    let record = dirs.staterc();
    let staterc = match fs::read_to_string(&record) {
        Ok(staterc) => staterc,
        Err(err) => {
            debug!(
                "no desktop layout to take: {} could not be read: {err}",
                record.display()
            );

            return None;
        }
    };

    let path = layout_file(dirs, &staterc)?;
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            warn!(
                "the desktop names {} as its bar layout, and it could not be read: {err}",
                path.display()
            );

            return None;
        }
    };

    let restated = parse(&source, custom_names);

    if restated.is_none() {
        warn!(
            "{} holds no entry this bar can place; the layout it already has is kept",
            path.display()
        );
    }

    restated
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

        debug!(
            "the recorded layout path {} is gone; falling back to the recorded name",
            path.display()
        );
    }

    let Some(name) = shell_vars::value_of(staterc, LAYOUT_NAME_KEY) else {
        debug!("the desktop records no bar layout to take");

        return None;
    };

    let path = dirs.bar_layouts_dir().join(format!("{name}.jsonc"));

    if !path.is_file() {
        warn!(
            "the desktop records the bar layout `{name}`, and no file answers to it at {}",
            path.display()
        );

        return None;
    }

    Some(path)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::config::{ModuleDef, ModuleName};

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

        let modules = load_from(&dirs, &[]).expect("layout").modules;

        assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Clock)]);
    }

    #[test]
    fn a_gone_path_falls_back_to_the_recorded_name() {
        let (_root, dirs) = install(
            "WAYBAR_LAYOUT_PATH=\"/nonexistent/gone.jsonc\"\nWAYBAR_LAYOUT_NAME=\"hyprdots/01\"\n",
            Some(("hyprdots/01.jsonc", r#"{ "modules-left": ["clock"] }"#))
        );

        let modules = load_from(&dirs, &[]).expect("layout").modules;

        assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Clock)]);
    }

    #[test]
    fn a_missing_install_answers_with_nothing() {
        let (_root, dirs) = install("", None);

        assert_eq!(load_from(&dirs, &[]), None);
    }
}
