//! Events emitted by the configuration watcher.

use std::ffi::{OsStr, OsString};

use inotify::EventMask;

use crate::config::{ConfigApplied, ConfigDegradation};

#[derive(Debug, Clone)]
pub enum ConfigEvent {
    /// A new, validated configuration was applied.
    Applied(ConfigApplied),
    /// The configuration could not be refreshed and the previous state is
    /// retained.
    Degraded(ConfigDegradation)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Event {
    Changed,
    Removed
}

pub(super) trait WatchedEvent {
    fn file_name(&self) -> Option<&OsStr>;

    fn mask(&self) -> EventMask;
}

impl WatchedEvent for inotify::Event<OsString> {
    fn file_name(&self) -> Option<&OsStr> {
        self.name.as_deref()
    }

    fn mask(&self) -> EventMask {
        self.mask
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WatchLoopOutcome {
    StreamEnded,
    HandlerClosed
}
