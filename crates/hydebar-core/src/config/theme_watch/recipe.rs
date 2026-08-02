//! Subscription recipe driving the `HyDE` theme watcher.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration
};

use iced::Subscription;
use iced_futures::subscription::from_recipe;

use super::sources::{ThemeRoots, ThemeWatchTarget};
use crate::config::{ConfigEvent, ConfigManager};

mod stream;
mod watches;

/// Number of events read from the kernel in one go.
///
/// A theme switch re-points the palette and rewrites the state file within
/// milliseconds of each other, and the consumers that follow add more noise, so
/// the batch exists to collapse that burst into a single reload.
const BATCH_SIZE: usize = 16;

/// How long the watcher lets the desktop settle before it reads the theme.
///
/// A `HyDE` switch is not one write but a chain of them spread over seconds:
/// the state file, the palette symlink, then every template the desktop renders
/// from the new colours. Reading on the first of them would repaint the bar
/// from a desktop that is half-way through changing, and reading on each of
/// them would repaint it a dozen times over. Waiting first turns the chain into
/// a couple of reloads, and each one sees a consistent set of files, because
/// everything that lands during the wait is still queued in the kernel and
/// arrives as one batch.
const SETTLE: Duration = Duration::from_millis(300);

struct ThemeWatcher {
    config_path: PathBuf,
    roots:       ThemeRoots,
    targets:     Vec<ThemeWatchTarget>,
    manager:     Arc<ConfigManager>
}

/// Follows the `HyDE` theme and republishes the configuration when it changes.
///
/// The bar reads its palette from files the configuration watcher never sees,
/// so without this a theme switch left the bar in the old colours until it was
/// restarted. Reloading the configuration rather than patching the appearance
/// in place keeps explicit settings winning over the theme, exactly as they do
/// on a cold start.
///
/// Nothing is watched when `follow_hyde` is false: the configuration then owns
/// its colours and a theme switch must not disturb them.
pub fn theme_subscription(
    config_path: &Path,
    manager: Arc<ConfigManager>,
    follow_hyde: bool
) -> Subscription<ConfigEvent> {
    /// The environment cannot change under a running process, and this
    /// derivation runs after every update batch — so the walk over the
    /// environment and the path joins happen exactly once. Only the roots
    /// are pinned: the follow flag stays live, a reload may flip it.
    static ROOTS: std::sync::OnceLock<Option<(ThemeRoots, Vec<ThemeWatchTarget>)>> =
        std::sync::OnceLock::new();

    if !follow_hyde {
        return Subscription::none();
    }

    let Some((roots, targets)) = ROOTS
        .get_or_init(|| ThemeRoots::from_env().map(|roots| (roots.clone(), roots.targets())))
        .as_ref()
    else {
        return Subscription::none();
    };

    if targets.is_empty() {
        return Subscription::none();
    }

    from_recipe(ThemeWatcher {
        config_path: config_path.to_path_buf(),
        roots: roots.clone(),
        targets: targets.clone(),
        manager
    })
}
