//! Subscription recipe driving the `HyDE` theme watcher.

use std::{
    any::TypeId,
    hash::Hash,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration
};

use iced::{
    Subscription,
    futures::{StreamExt, channel::mpsc::Sender, pin_mut},
    stream::channel
};
use iced_futures::subscription::{self, Recipe, from_recipe};
use inotify::{Inotify, WatchMask};
use log::{debug, error, info, warn};

use super::{
    interpret::{handle_theme_event, interpret_theme_event},
    sources::{ThemeRoots, ThemeWatchTarget, watch_targets, watched_names}
};
use crate::config::{
    ConfigEvent, ConfigManager,
    watch::{WatchLoopOutcome, interpret::process_event_batches}
};

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

impl Recipe for ThemeWatcher {
    type Output = ConfigEvent;

    fn hash(&self, state: &mut subscription::Hasher) {
        TypeId::of::<Self>().hash(state);
        self.config_path.hash(state);
        self.roots.hash(state);
        Arc::as_ptr(&self.manager).hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream
    ) -> iced::futures::stream::BoxStream<'static, Self::Output> {
        let Self {
            config_path,
            roots,
            targets,
            manager
        } = *self;

        Box::pin(channel(100, move |output: Sender<ConfigEvent>| {
            let manager = Arc::clone(&manager);
            let names = watched_names();

            async move {
                loop {
                    let inotify = match Inotify::init() {
                        Ok(inotify) => inotify,
                        Err(e) => {
                            error!("Failed to initialize inotify for the HyDE theme: {e}");
                            break;
                        }
                    };

                    if !add_watches(&inotify, &targets) {
                        error!("No HyDE theme directory could be watched; giving up");
                        break;
                    }

                    let buffer = [0; 1024];
                    let stream = match inotify.into_event_stream(buffer) {
                        Ok(stream) => stream,
                        Err(e) => {
                            error!("Failed to create the HyDE theme event stream: {e}");
                            break;
                        }
                    };

                    let event_stream = stream.ready_chunks(BATCH_SIZE);
                    pin_mut!(event_stream);

                    let sender_template = output.clone();
                    let config_path_clone = config_path.clone();
                    let roots_clone = roots.clone();
                    let manager_clone = Arc::clone(&manager);
                    let names_ref = &names;

                    let outcome = process_event_batches(
                        event_stream.as_mut(),
                        |event| interpret_theme_event(event, names_ref),
                        move |event| {
                            let mut sender = sender_template.clone();
                            let config_path = config_path_clone.clone();
                            let roots = roots_clone.clone();
                            let manager = Arc::clone(&manager_clone);

                            async move {
                                tokio::time::sleep(SETTLE).await;

                                handle_theme_event(
                                    &mut sender,
                                    &config_path,
                                    &roots,
                                    event,
                                    manager
                                )
                                .await
                            }
                        }
                    )
                    .await;

                    match outcome {
                        WatchLoopOutcome::StreamEnded => {
                            info!(
                                "HyDE theme watch stream closed; attempting to restart the inotify watcher"
                            );
                        }
                        WatchLoopOutcome::HandlerClosed => {
                            info!("HyDE theme watch handler closed; stopping watcher loop");
                            break;
                        }
                    }
                }

                info!("HyDE theme watcher terminated");
            }
        }))
    }
}

/// Places a watch on every theme directory, reporting whether any took.
///
/// A missing directory is not fatal: a session may have no `HyDE` cache yet
/// while its state file exists, and the theme is still worth following through
/// the directories that do exist.
fn add_watches(inotify: &Inotify, targets: &[ThemeWatchTarget]) -> bool {
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
    let targets = watch_targets(follow_hyde);

    if targets.is_empty() {
        return Subscription::none();
    }

    let Some(roots) = ThemeRoots::from_env() else {
        return Subscription::none();
    };

    from_recipe(ThemeWatcher {
        config_path: config_path.to_path_buf(),
        roots,
        targets,
        manager
    })
}
