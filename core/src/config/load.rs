//! Locating the configuration file and loading it with a safety net.
//!
//! The path the user gave — or the default location when none was — is
//! shell-expanded, checked for existence and, for the default location, has
//! its directory created on first run. Whatever the file then holds, the bar
//! starts: a file that fails to read, parse or validate is logged and answered
//! with the default configuration instead of a refusal to launch.

use std::{
    fs,
    path::{Path, PathBuf}
};

use hydebar_proto::config::{Config, DEFAULT_CONFIG_FILE_PATH};
use log::{info, warn};
use shellexpand::full;

use super::read::read_config;

/// Why the configuration file could not be opened.
#[derive(Debug)]
pub enum ConfigLoadError {
    /// The path could not be expanded.
    Expand {
        /// The path as it was written.
        input:  String,
        /// What the expansion tripped over.
        source: shellexpand::LookupError<std::env::VarError>
    },
    /// There is no file at that path.
    Missing {
        /// The path that was looked at.
        path: PathBuf
    },
    /// The directory that would hold the file does not exist.
    MissingParent {
        /// The path that was looked at.
        path: PathBuf
    },
    /// The directory could not be made.
    CreateDir {
        /// The directory that was to be made.
        path:   PathBuf,
        /// What the operating system said.
        source: std::io::Error
    }
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expand {
                input,
                source
            } => {
                write!(f, "failed to expand config path '{input}': {source}")
            }
            Self::Missing {
                path
            } => {
                write!(f, "config file does not exist: {}", path.display())
            }
            Self::MissingParent {
                path
            } => {
                write!(
                    f,
                    "config path '{}' has no parent directory",
                    path.display()
                )
            }
            Self::CreateDir {
                path,
                source
            } => {
                write!(
                    f,
                    "failed to create config directory '{}': {}",
                    path.display(),
                    source
                )
            }
        }
    }
}

impl std::error::Error for ConfigLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Expand {
                source, ..
            } => Some(source),
            Self::CreateDir {
                source, ..
            } => Some(source),
            _ => None
        }
    }
}

/// Loads the configuration from `path`, or from the default location when no
/// path is given.
///
/// # Errors
///
/// Returns a [`ConfigLoadError`] when the path cannot be expanded, the given
/// file does not exist, or the default configuration directory cannot be
/// resolved or created.
pub fn get_config(path: Option<PathBuf>) -> Result<(Config, PathBuf), ConfigLoadError> {
    if let Some(path) = path {
        info!("Config path provided {}", path.display());
        let expanded = expand_path(&path)?;

        if !expanded.exists() {
            return Err(ConfigLoadError::Missing {
                path: expanded
            });
        }

        let config = load_config_or_default(&expanded);

        Ok((config, expanded))
    } else {
        let expanded = expand_path(Path::new(DEFAULT_CONFIG_FILE_PATH))?;
        ensure_parent_exists(&expanded)?;

        let config = load_config_or_default(&expanded);

        Ok((config, expanded))
    }
}

fn expand_path(path: &Path) -> Result<PathBuf, ConfigLoadError> {
    let input = path.to_string_lossy().into_owned();
    match full(&input) {
        Ok(expanded) => Ok(PathBuf::from(expanded.to_string())),
        Err(source) => Err(ConfigLoadError::Expand {
            input,
            source
        })
    }
}

fn ensure_parent_exists(path: &Path) -> Result<(), ConfigLoadError> {
    let parent = path
        .parent()
        .ok_or_else(|| ConfigLoadError::MissingParent {
            path: path.to_path_buf()
        })?;

    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|source| ConfigLoadError::CreateDir {
            path: parent.to_path_buf(),
            source
        })?;
    }

    Ok(())
}

fn load_config_or_default(path: &Path) -> Config {
    info!("Decoding config file {}", path.display());

    match read_config(path) {
        Ok(config) => match config.validate() {
            Ok(()) => {
                info!("Config file loaded successfully");
                config
            }
            Err(err) => {
                warn!("{err}");
                warn!("Falling back to default configuration");
                Config::default()
            }
        },
        Err(err) => {
            warn!("{err}");
            warn!("Falling back to default configuration");
            Config::default()
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn get_config_returns_default_on_parse_error() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "invalid = [").expect("failed to write invalid config");

        let (config, returned_path) =
            get_config(Some(config_path.clone())).expect("get_config should succeed");

        assert_eq!(returned_path, config_path);
        let default = Config::default();
        assert_eq!(config.log_level, default.log_level);
        assert_eq!(config.menu_keyboard_focus, default.menu_keyboard_focus);
        assert_eq!(config.position, default.position);
    }

    #[test]
    fn get_config_errors_when_file_missing() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("missing.toml");

        let error = get_config(Some(config_path.clone())).expect_err("expected error");

        match error {
            ConfigLoadError::Missing {
                path
            } => assert_eq!(path, config_path),
            other => panic!("unexpected error: {other:?}")
        }
    }
}
