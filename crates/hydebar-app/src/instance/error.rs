//! Failures that keep the process from becoming the single instance.
//!
//! Every variant names the step that failed — preparing the runtime
//! directory, opening or locking the file, signalling the incumbent, or
//! waiting it out — so the startup error the user sees points at the exact
//! obstacle rather than a generic refusal.

use std::{io, path::PathBuf, time::Duration};

/// Reason the process could not become the single instance.
#[derive(Debug)]
pub enum InstanceError {
    /// The runtime directory holding the lock file could not be prepared.
    Directory(PathBuf, io::Error),
    /// The lock file could not be opened.
    Open(PathBuf, io::Error),
    /// Locking the file failed for a reason other than contention.
    Lock(PathBuf, io::Error),
    /// The owner process could not be signalled to quit.
    Signal(i32, io::Error),
    /// The previous instance still held the lock when the wait ran out.
    Timeout(Option<i32>, Duration)
}

impl std::fmt::Display for InstanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Directory(path, err) => {
                write!(
                    f,
                    "failed to create the runtime directory {}: {err}",
                    path.display()
                )
            }
            Self::Open(path, err) => {
                write!(f, "failed to open the lock file {}: {err}", path.display())
            }
            Self::Lock(path, err) => write!(f, "failed to lock {}: {err}", path.display()),
            Self::Signal(pid, err) => {
                write!(f, "failed to ask the running instance {pid} to quit: {err}")
            }
            Self::Timeout(Some(pid), waited) => write!(
                f,
                "the running instance {pid} did not quit within {} ms, refusing to draw a second \
                 bar",
                waited.as_millis()
            ),
            Self::Timeout(None, waited) => write!(
                f,
                "another instance kept the lock for {} ms, refusing to draw a second bar",
                waited.as_millis()
            )
        }
    }
}

impl std::error::Error for InstanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Directory(_, err)
            | Self::Open(_, err)
            | Self::Lock(_, err)
            | Self::Signal(_, err) => Some(err),
            Self::Timeout(..) => None
        }
    }
}
