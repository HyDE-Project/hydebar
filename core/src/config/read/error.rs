//! The failures one configuration read can report.

use std::path::PathBuf;

/// A configuration file that could not be read or could not be parsed.
#[derive(Debug)]
pub enum ConfigReadError {
    /// The file could not be opened or read.
    Read {
        /// The path the bar tried to read.
        path:   PathBuf,
        /// The failure reported by the operating system.
        source: std::io::Error
    },
    /// The file was read but is not a configuration.
    Parse {
        /// The path whose contents failed to parse.
        path:   PathBuf,
        /// The failure reported by the parser.
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
