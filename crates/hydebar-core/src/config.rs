use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf}
};

pub use hydebar_proto::config::*;
use hydebar_proto::{
    bar_layout,
    theme_source::{self, HydeTheme}
};

pub mod manager;
pub mod theme_watch;
pub mod watch;

use log::{info, warn};
pub use manager::{
    ConfigApplied, ConfigDegradation, ConfigImpact, ConfigManager, ConfigUpdateError
};
use shellexpand::full;
pub use theme_watch::{ThemeRoots, ThemeWatchTarget, theme_subscription};
pub use watch::{ConfigEvent, subscription};

#[derive(Debug)]
pub enum ConfigLoadError {
    Expand {
        input:  String,
        source: shellexpand::LookupError<std::env::VarError>
    },
    Missing {
        path: PathBuf
    },
    MissingParent {
        path: PathBuf
    },
    CreateDir {
        path:   PathBuf,
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
                write!(f, "failed to expand config path '{}': {}", input, source)
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

#[derive(Debug)]
pub(crate) enum ConfigReadError {
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

pub fn get_config(path: Option<PathBuf>) -> Result<(Config, PathBuf), ConfigLoadError> {
    match path {
        Some(path) => {
            info!("Config path provided {path:?}");
            let expanded = expand_path(path)?;

            if !expanded.exists() {
                return Err(ConfigLoadError::Missing {
                    path: expanded
                });
            }

            let config = load_config_or_default(&expanded);

            Ok((config, expanded))
        }
        None => {
            let expanded = expand_path(PathBuf::from(DEFAULT_CONFIG_FILE_PATH))?;
            ensure_parent_exists(&expanded)?;

            let config = load_config_or_default(&expanded);

            Ok((config, expanded))
        }
    }
}

fn expand_path(path: PathBuf) -> Result<PathBuf, ConfigLoadError> {
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

pub(crate) fn read_config(path: &Path) -> Result<Config, ConfigReadError> {
    read_config_with(path, theme_source::load, bar_layout::load)
}

/// Reads the configuration and overlays what HyDE answers for.
///
/// The theme and the layout are closures rather than values because reading
/// them touches the disk and a configuration that opts out of following HyDE
/// must not pay for either. Injecting them also lets a watcher overlay the
/// directory it is watching, which is the only way a test can observe a theme
/// switch without mutating the environment of the whole process.
pub(crate) fn read_config_with<F, G>(
    path: &Path,
    theme: F,
    layout: G
) -> Result<Config, ConfigReadError>
where
    F: FnOnce() -> HydeTheme,
    G: FnOnce(&[String]) -> Option<Modules>
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

    table
        .try_into()
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
/// with anything HyDE has on file.
fn declares_modules(table: &toml::Table) -> bool {
    table.contains_key("modules")
}

/// Overlays the HyDE theme and bar layout onto a freshly parsed configuration.
///
/// The overlay runs *after* the file has been parsed and only fills what the
/// user left unset, which is what fixes the precedence for good: what is
/// written in `~/.config/hydebar/config.toml` wins, what HyDE says fills the
/// rest, and the bar's own defaults answer for whatever is left. No theme or
/// layout switch can undo a value the user wrote.
///
/// Runs on every read, so a hot reload picks up a switch that happened while
/// the bar was running. Opting out with `appearance.follow_hyde = false`
/// skips reading HyDE entirely.
fn follow_hyde<F, G>(mut config: Config, manual_layout: bool, theme: F, layout: G) -> Config
where
    F: FnOnce() -> HydeTheme,
    G: FnOnce(&[String]) -> Option<Modules>
{
    if !config.appearance.follow_hyde {
        return config;
    }

    config.appearance.apply_hyde_theme(&theme());

    if !manual_layout {
        let custom_names: Vec<String> = config
            .custom_modules
            .iter()
            .map(|definition| definition.name.clone())
            .collect();

        if let Some(modules) = layout(&custom_names) {
            config.modules = modules;
        }
    }

    config
}

fn load_config_or_default(path: &Path) -> Config {
    info!("Decoding config file {path:?}");

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

    fn hyde_layout(_custom_names: &[String]) -> Option<Modules> {
        Some(Modules {
            left:   vec![ModuleDef::Single(ModuleName::Clock)],
            center: Vec::new(),
            right:  Vec::new()
        })
    }

    /// A configuration that writes no module layout takes the one HyDE has on
    /// file, exactly as the bar HyDE started with would.
    #[test]
    fn an_undeclared_layout_follows_the_desktop() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "position = \"Top\"\n").expect("config");

        let config =
            read_config_with(&config_path, HydeTheme::default, hyde_layout).expect("config reads");

        assert_eq!(
            config.modules.left,
            vec![ModuleDef::Single(ModuleName::Clock)]
        );
        assert!(config.modules.center.is_empty());
    }

    /// A hand-written `[modules]` section is manual control and no desktop
    /// layout may displace it.
    #[test]
    fn a_declared_layout_outranks_the_desktop() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "[modules]\nleft = [\"Battery\"]\n").expect("config");

        let config =
            read_config_with(&config_path, HydeTheme::default, hyde_layout).expect("config reads");

        assert_eq!(
            config.modules.left,
            vec![ModuleDef::Single(ModuleName::Battery)]
        );
    }

    /// Opting out of following HyDE opts out of its layout with it.
    #[test]
    fn opting_out_of_hyde_keeps_the_default_layout() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "[appearance]\nfollow_hyde = false\n").expect("config");

        let config =
            read_config_with(&config_path, HydeTheme::default, hyde_layout).expect("config reads");

        assert_eq!(config.modules, Modules::default());
    }

    /// A desktop with no readable layout leaves the default in place.
    #[test]
    fn a_missing_desktop_layout_leaves_the_default() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "position = \"Top\"\n").expect("config");

        let config =
            read_config_with(&config_path, HydeTheme::default, |_| None).expect("config reads");

        assert_eq!(config.modules, Modules::default());
    }
}
