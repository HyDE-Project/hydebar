//! Reading and parsing the configuration file from disk.
//!
//! One read is one pass: the file is read to a string, parsed as TOML, a
//! named theme is stood in for its file when one answers beside the
//! configuration, and the result is handed to the `HyDE` overlay in
//! [`super::hyde`] before it is returned.
//! Whether the file declared a `[modules]` section of its own is noted while
//! the raw table is still in hand, because that presence — not its content —
//! is what pins the layout against the desktop's.

use std::{fs::File, io::Read, path::Path};

use hydebar_proto::{
    bar_layout,
    config::Config,
    theme_source::{self, HydeTheme}
};

use self::{
    theme::resolve_named_theme, theme_base::resolve_theme_base, unknown::read_naming_unknown
};
use super::hyde::follow_hyde;

mod error;
pub(super) mod theme;
mod theme_base;
mod unknown;

pub use error::ConfigReadError;

pub fn read_config(path: &Path) -> Result<Config, ConfigReadError> {
    read_config_with(path, theme_source::load, bar_layout::load)
}

/// Reads the configuration and overlays what `HyDE` answers for.
///
/// The theme and the layout are closures rather than values because reading
/// them touches the disk and a configuration that opts out of following `HyDE`
/// must not pay for either. Injecting them also lets a watcher overlay the
/// directory it is watching, which is the only way a test can observe a theme
/// switch without mutating the environment of the whole process.
pub fn read_config_with<F, G>(path: &Path, theme: F, layout: G) -> Result<Config, ConfigReadError>
where
    F: FnOnce() -> HydeTheme,
    G: FnOnce(&[String]) -> Option<bar_layout::RestatedLayout>
{
    let mut content = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut content))
        .map_err(|source| ConfigReadError::Read {
            path: path.to_path_buf(),
            source
        })?;

    let mut table: toml::Table =
        toml::from_str(&content).map_err(|source| ConfigReadError::Parse {
            path: path.to_path_buf(),
            source
        })?;
    resolve_named_theme(&mut table, path)?;
    resolve_theme_base(&mut table, path)?;
    let declared = declares_modules(&table);

    read_naming_unknown(table, path)
        .map(|config| follow_hyde(config, declared, theme, layout))
        .map_err(|source| ConfigReadError::Parse {
            path: path.to_path_buf(),
            source
        })
}

/// Whether the file writes a module layout of its own.
///
/// Presence is what matters, not content: a hand-written `[modules]` section
/// is manual control and pins the layout, however much it happens to coincide
/// with anything `HyDE` has on file.
fn declares_modules(table: &toml::Table) -> bool {
    table.contains_key("modules")
}
