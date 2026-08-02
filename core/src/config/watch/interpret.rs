//! Translation of inotify events into configuration updates.

use std::{
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc
};

use iced::futures::{
    SinkExt, Stream, StreamExt,
    channel::mpsc::{SendError, Sender}
};
use inotify::EventMask;
use log::{debug, error, info, warn};

use super::{
    ConfigEvent, Event, WatchLoopOutcome, WatchedEvent,
    load::{load_candidate, send_degradation}
};
use crate::config::{ConfigManager, ConfigUpdateError};

/// Classifies an inotify mask without looking at the file it belongs to.
///
/// The name check is the only part that differs between the watchers, so the
/// mask rules live on their own: a watcher following a whole set of files
/// reuses the very same notion of "replaced" the single file watcher uses,
/// instead of restating it and drifting away from it.
pub fn classify_mask(mask: EventMask) -> Option<Event> {
    let is_removed = mask.contains(EventMask::DELETE) || mask.contains(EventMask::MOVED_FROM);

    if is_removed && !mask.intersects(EventMask::CREATE | EventMask::MODIFY | EventMask::MOVED_TO)
    {
        debug!("File deleted or moved");
        return Some(Event::Removed);
    }

    let is_changed = mask.intersects(
        EventMask::CREATE | EventMask::MODIFY | EventMask::MOVED_TO | EventMask::CLOSE_WRITE
    );

    if is_changed {
        debug!("File changed");
        Some(Event::Changed)
    } else {
        None
    }
}

/// Reports how an event on the watched directory affects `target_name`.
pub fn interpret_event<E: WatchedEvent>(event: &E, target_name: &OsStr) -> Option<Event> {
    let name = event.file_name()?;

    if name != target_name {
        return None;
    }

    classify_mask(event.mask())
}

/// Drives a watch stream, folding every batch into at most one reload.
///
/// `classify` decides whether an event concerns the watcher at all, which is
/// what lets a single loop serve both the configuration file and the set of
/// files the desktop theme is spread across. Collapsing a batch into one call
/// matters because a theme switch rewrites several files at once and the bar
/// should repaint once, not once per file.
pub async fn process_event_batches<S, E, Err, C, F, Fut>(
    mut stream: Pin<&mut S>,
    classify: C,
    mut handler: F
) -> WatchLoopOutcome
where
    S: Stream<Item = Vec<Result<E, Err>>>,
    E: WatchedEvent + std::fmt::Debug,
    Err: Display,
    C: Fn(&E) -> Option<Event>,
    F: FnMut(Event) -> Fut,
    Fut: Future<Output = Result<(), SendError>>
{
    while let Some(batch) = stream.as_mut().next().await {
        let mut file_event = None;

        for event in batch {
            match event {
                Ok(event) => {
                    debug!("Event: {event:?}");

                    match classify(&event) {
                        Some(kind) => {
                            file_event = Some(kind);
                        }
                        None => {
                            debug!("Ignoring event");
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to read watch event: {err}");
                }
            }
        }

        if let Some(kind) = file_event {
            if let Err(err) = handler(kind).await {
                warn!("Stopping config watch because handler returned an error: {err}");
                return WatchLoopOutcome::HandlerClosed;
            }
        } else {
            debug!("No relevant file event detected.");
        }
    }

    WatchLoopOutcome::StreamEnded
}

/// Loads the candidate on the blocking pool, mirroring the theme watcher.
///
/// The load opens and parses the file, reads the theme sources and may ask
/// the compositor for its look — all blocking work that would otherwise park
/// a runtime worker for the duration, in the middle of the reload burst that
/// triggered it.
async fn load_candidate_off_thread(
    path: PathBuf,
    manager: Arc<ConfigManager>
) -> Result<crate::config::ConfigApplied, ConfigUpdateError> {
    let read = tokio::task::spawn_blocking(move || load_candidate(&path, &manager)).await;

    match read {
        Ok(outcome) => outcome,
        Err(error) => Err(ConfigUpdateError::state(format!(
            "the configuration could not be read: {error}"
        )))
    }
}

/// Applies a watch event to the configuration and reports the outcome.
pub async fn handle_watch_event(
    output: &mut Sender<ConfigEvent>,
    path: &Path,
    event: Event,
    manager: Arc<ConfigManager>
) -> Result<(), SendError> {
    match event {
        Event::Changed => {
            info!("Reload config file");

            match load_candidate_off_thread(path.to_owned(), Arc::clone(&manager)).await {
                Ok(applied) => output.send(ConfigEvent::Applied(applied)).await,
                Err(reason) => {
                    warn!("Configuration update failed: {reason}");
                    send_degradation(output, manager, reason).await
                }
            }
        }
        Event::Removed => {
            info!("Config file removed");

            send_degradation(output, manager, ConfigUpdateError::Removed).await
        }
    }
}
