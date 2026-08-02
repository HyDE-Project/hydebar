//! Placement of the inotify watches on the theme directories.

use inotify::{Inotify, WatchMask};
use log::{debug, warn};

use crate::config::theme_watch::sources::ThemeWatchTarget;

/// Places a watch on every theme directory, reporting whether any took.
///
/// A missing directory is not fatal: a session may have no `HyDE` cache yet
/// while its state file exists, and the theme is still worth following through
/// the directories that do exist.
pub(super) fn add_watches(inotify: &Inotify, targets: &[ThemeWatchTarget]) -> bool {
    let mask = WatchMask::CREATE
        | WatchMask::DELETE
        | WatchMask::MOVE
        | WatchMask::MODIFY
        | WatchMask::CLOSE_WRITE;

    let mut watched = false;

    for target in targets {
        match inotify.watches().add(&target.directory, mask) {
            Ok(_) => {
                debug!(
                    "Watching HyDE theme directory {}",
                    target.directory.display()
                );
                watched = true;
            }
            Err(e) => {
                warn!(
                    "Failed to watch the HyDE theme directory {}: {e}",
                    target.directory.display()
                );
            }
        }
    }

    watched
}
