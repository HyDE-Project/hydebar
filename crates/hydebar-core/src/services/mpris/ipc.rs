//! Event stream plumbing for MPRIS players on the session bus.

use std::{pin::Pin, sync::Arc};

use futures::{Stream, StreamExt, stream::SelectAll};
use masterror::{AppError, AppResult};
use zbus::{Connection, fdo::DBusProxy};

use super::data::{MprisPlayerMetadata, PlaybackStatus};

mod players;

pub use players::{collect_players, fetch_players, is_mpris_service};

/// Stream item emitted by [`build_event_stream`].
#[derive(Debug)]
pub enum IpcEvent {
    /// Indicates that the ownership of an MPRIS name changed.
    NameOwner,
    /// Metadata for `service` changed.
    Metadata(String, Option<MprisPlayerMetadata>),
    /// Volume for `service` changed.
    Volume(String, Option<f64>),
    /// Playback state for `service` changed.
    State(String, PlaybackStatus)
}

/// Combined event stream type returned by [`build_event_stream`].
pub type EventStream = SelectAll<Pin<Box<dyn Stream<Item = IpcEvent> + Send>>>;

/// Builds a stream that emits [`IpcEvent`] values for all active players.
pub async fn build_event_stream(conn: &Connection) -> AppResult<EventStream> {
    let dbus = DBusProxy::new(conn)
        .await
        .map_err(|e| AppError::internal(format!("Failed to create DBusProxy: {e}")))?;
    let data = collect_players(conn).await?;
    let mut combined = SelectAll::new();

    combined.push(Box::pin(
        dbus.receive_name_owner_changed()
            .await
            .map_err(|e| AppError::internal(format!("Failed to receive name owner changed: {e}")))?
            .filter_map(|signal| async move {
                match signal.args() {
                    Ok(args) if is_mpris_service(&args.name) => Some(IpcEvent::NameOwner),
                    _ => None
                }
            })
    ) as Pin<Box<dyn Stream<Item = IpcEvent> + Send>>);

    for entry in &data {
        let cache = Arc::new(entry.metadata.clone());
        let service = entry.service.clone();

        combined.push(
            Box::pin(entry.proxy.receive_metadata_changed().await.filter_map({
                let cache = Arc::clone(&cache);
                let service = service.clone();

                move |metadata| {
                    let cache = Arc::clone(&cache);
                    let service = service.clone();

                    async move {
                        let new_metadata =
                            metadata.get().await.map(MprisPlayerMetadata::from).ok();

                        if new_metadata.as_ref() == cache.as_ref().as_ref() {
                            None
                        } else {
                            Some(IpcEvent::Metadata(service, new_metadata))
                        }
                    }
                }
            })) as Pin<Box<dyn Stream<Item = IpcEvent> + Send>>
        );
    }

    for entry in &data {
        let service = entry.service.clone();
        let volume = entry.volume;

        combined.push(
            Box::pin(
                entry
                    .proxy
                    .receive_volume_changed()
                    .await
                    .filter_map(move |signal| {
                        let service = service.clone();

                        async move {
                            let new_volume = signal.get().await.ok();
                            if new_volume == volume {
                                None
                            } else {
                                Some(IpcEvent::Volume(service, new_volume))
                            }
                        }
                    })
            ) as Pin<Box<dyn Stream<Item = IpcEvent> + Send>>
        );
    }

    for entry in &data {
        let service = entry.service.clone();
        let state = entry.state;

        combined.push(Box::pin(
            entry
                .proxy
                .receive_playback_status_changed()
                .await
                .filter_map(move |signal| {
                    let service = service.clone();

                    async move {
                        let new_state = signal
                            .get()
                            .await
                            .map(PlaybackStatus::from)
                            .unwrap_or_default();

                        if new_state == state {
                            None
                        } else {
                            Some(IpcEvent::State(service, new_state))
                        }
                    }
                })
        ) as Pin<Box<dyn Stream<Item = IpcEvent> + Send>>);
    }

    Ok(combined)
}
