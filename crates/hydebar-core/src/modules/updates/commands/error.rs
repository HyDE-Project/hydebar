//! What a shell command can fail with, and how far a failure reaches.

use std::process::ExitStatus;

/// Errors that can occur while executing an update-related shell command.
#[derive(Debug)]
pub enum CommandError {
    /// Failed to spawn the command.
    Io(std::io::Error),
    /// The command exited with a non-zero status.
    Status(ExitStatus)
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(_) => write!(f, "failed to execute command"),
            Self::Status(status) => write!(f, "command exited with failure status: {status}")
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Status(_) => None
        }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl CommandError {
    pub fn or_log(self, context: &str) {
        log::warn!("{context}: {self}");
    }
}

/// Why a check could not be trusted.
#[derive(Debug)]
pub enum CheckFailure {
    /// The check cannot be run on this machine at all.
    ///
    /// A configuration naming a package manager the machine does not have
    /// is not a fault to report every hour; it means the bar has
    /// nothing to show.
    Unavailable(CommandError),
    /// The check ran but this particular run said nothing usable.
    Transient(CommandError)
}

impl std::fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(err) | Self::Transient(err) => write!(f, "{err}")
        }
    }
}
