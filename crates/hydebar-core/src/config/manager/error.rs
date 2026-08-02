//! Failure modes of a configuration refresh.
//!
//! A refresh can fail on the way in — reading, parsing or validating the file
//! — or the file can vanish outright; every such outcome is one
//! [`ConfigUpdateError`] variant. A failed refresh degrades rather than
//! crashes: [`ConfigDegradation`] pairs the reason with the last configuration
//! that did apply, so the bar keeps running on known-good state.

use std::path::PathBuf;

use hydebar_proto::config::{Config, ConfigValidationError};

/// Describes failures that occurred while attempting to refresh the
/// configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigUpdateError {
    /// Reading the configuration file from disk failed.
    Read { path: PathBuf, context: String },
    /// Parsing TOML content failed.
    Parse { path: PathBuf, context: String },
    /// Validation detected a logical inconsistency.
    Validation(ConfigValidationError),
    /// The configuration file was removed.
    Removed,
    /// Updating the configuration state failed for an internal reason.
    State { context: String }
}

impl std::fmt::Display for ConfigUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read {
                path,
                context
            } => {
                write!(f, "failed to read config at {}: {context}", path.display())
            }
            Self::Parse {
                path,
                context
            } => {
                write!(f, "failed to parse config at {}: {context}", path.display())
            }
            Self::Validation(err) => write!(f, "{err}"),
            Self::Removed => write!(f, "configuration file removed"),
            Self::State {
                context
            } => {
                write!(f, "failed to update configuration state: {context}")
            }
        }
    }
}

impl std::error::Error for ConfigUpdateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Validation(err) => Some(err),
            _ => None
        }
    }
}

impl From<ConfigValidationError> for ConfigUpdateError {
    fn from(err: ConfigValidationError) -> Self {
        Self::Validation(err)
    }
}

impl ConfigUpdateError {
    /// Construct a read error with contextual information.
    #[must_use]
    pub fn read(path: PathBuf, err: &std::io::Error) -> Self {
        Self::Read {
            path,
            context: err.to_string()
        }
    }

    /// Construct a parse error with contextual information.
    #[must_use]
    pub fn parse(path: PathBuf, err: &toml::de::Error) -> Self {
        Self::Parse {
            path,
            context: err.to_string()
        }
    }

    /// Construct a state management error.
    pub fn state(context: impl Into<String>) -> Self {
        Self::State {
            context: context.into()
        }
    }
}

/// Information about configuration degradation events.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigDegradation {
    /// The reason the configuration could not be refreshed.
    pub reason:     ConfigUpdateError,
    /// The last known valid configuration.
    pub last_valid: Box<Config>
}

/// Errors produced by [`super::ConfigManager`].
#[derive(Debug)]
pub enum ConfigManagerError {
    /// The internal configuration state lock was poisoned.
    Poisoned
}

impl std::fmt::Display for ConfigManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => write!(f, "config state lock poisoned")
        }
    }
}

impl std::error::Error for ConfigManagerError {}
