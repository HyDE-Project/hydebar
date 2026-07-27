//! Translation of inotify events into configuration updates.

use std::{ffi::OsStr, fmt::Display, path::Path, pin::Pin, sync::Arc};

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

pub(super) fn interpret_event<E: WatchedEvent>(event: &E, target_name: &OsStr) -> Option<Event> {
    let name = event.file_name()?;

    if name != target_name {
        return None;
    }

    let mask = event.mask();

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

pub(super) async fn process_event_batches<S, E, Err, F, Fut>(
    mut stream: Pin<&mut S>,
    target_name: &OsStr,
    mut handler: F
) -> WatchLoopOutcome
where
    S: Stream<Item = Vec<Result<E, Err>>>,
    E: WatchedEvent + std::fmt::Debug,
    Err: Display,
    F: FnMut(Event) -> Fut,
    Fut: Future<Output = Result<(), SendError>>
{
    while let Some(batch) = stream.as_mut().next().await {
        let mut file_event = None;

        for event in batch {
            match event {
                Ok(event) => {
                    debug!("Event: {event:?}");

                    match interpret_event(&event, target_name) {
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

pub(super) async fn handle_watch_event(
    output: &mut Sender<ConfigEvent>,
    path: &Path,
    event: Event,
    manager: Arc<ConfigManager>
) -> Result<(), SendError> {
    match event {
        Event::Changed => {
            info!("Reload config file");

            match load_candidate(path, &manager) {
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
