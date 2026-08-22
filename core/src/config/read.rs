//! Reading and parsing the configuration file from disk.
//!
//! One read is one pass: the file is read to a string, parsed as TOML, and
//! handed to the `HyDE` overlay in [`super::hyde`] before it is returned.
//! Whether the file declared a `[modules]` section of its own is noted while
//! the raw table is still in hand, because that presence — not its content —
//! is what pins the layout against the desktop's.

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf}
};

use hydebar_proto::{
    bar_layout,
    config::Config,
    theme_source::{self, HydeTheme}
};

use self::unknown::read_naming_unknown;
use super::hyde::follow_hyde;

mod unknown;

#[derive(Debug)]
pub enum ConfigReadError {
    Read {
        path:   PathBuf,
        source: std::io::Error
    },
    Parse {
        path:   PathBuf,
        source: toml::de::Error
    }
}

impl std::fmt::Display for ConfigReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read {
                path,
                source
            } => {
                write!(
                    f,
                    "failed to read config file '{}': {}",
                    path.display(),
                    source
                )
            }
            Self::Parse {
                path,
                source
            } => {
                write!(
                    f,
                    "failed to parse config file '{}': {}",
                    path.display(),
                    source
                )
            }
        }
    }
}

impl std::error::Error for ConfigReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read {
                source, ..
            } => Some(source),
            Self::Parse {
                source, ..
            } => Some(source)
        }
    }
}

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

    let table: toml::Table =
        toml::from_str(&content).map_err(|source| ConfigReadError::Parse {
            path: path.to_path_buf(),
            source
        })?;
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
