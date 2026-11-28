//! Event stream plumbing for MPRIS players on the session bus.

use std::{pin::Pin, sync::Arc};

use futures::{Stream, StreamExt, stream::SelectAll};
use masterror::AppResult;
use zbus::{Connection, fdo::DBusProxy};

use super::data::{MprisPlayerData, MprisPlayerMetadata, PlaybackStatus};

mod players;

pub use players::{collect_players, fetch_players, is_mpris_service};

use crate::services::bus::bus_failure;

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

/// One subscription to one player property, shaped into [`IpcEvent`]s.
type BoxedStream = Pin<Box<dyn Stream<Item = IpcEvent> + Send>>;

/// Builds a stream that emits [`IpcEvent`] values for all active players.
pub async fn build_event_stream(conn: &Connection) -> AppResult<EventStream> {
    let dbus = DBusProxy::new(conn)
        .await
        .map_err(|e| bus_failure("Failed to create DBusProxy", &e))?;
    let data = collect_players(conn).await?;
    let mut combined = SelectAll::new();

    combined.push(name_owner_stream(&dbus).await?);

    for entry in &data {
        combined.push(metadata_stream(entry).await);
    }

    for entry in &data {
        combined.push(volume_stream(entry).await);
    }

    for entry in &data {
        combined.push(playback_stream(entry).await);
    }

    Ok(combined)
}

/// Fires when any MPRIS name changes owner on the bus.
async fn name_owner_stream(dbus: &DBusProxy<'_>) -> AppResult<BoxedStream> {
    Ok(dbus
        .receive_name_owner_changed()
        .await
        .map_err(|e| bus_failure("Failed to receive name owner changed", &e))?
        .filter_map(|signal| async move {
            match signal.args() {
                Ok(args) if is_mpris_service(&args.name) => Some(IpcEvent::NameOwner),
                _ => None
            }
        })
        .boxed())
}

/// Emits when a player's metadata moves off the cached one.
async fn metadata_stream(entry: &MprisPlayerData) -> BoxedStream {
    let cache = Arc::new(entry.metadata.clone());
    let service = entry.service.clone();

    entry
        .proxy
        .receive_metadata_changed()
        .await
        .filter_map(move |metadata| {
            let cache = Arc::clone(&cache);
            let service = service.clone();

            async move {
                let new_metadata = metadata.get().await.map(MprisPlayerMetadata::from).ok();

                if new_metadata.as_ref() == cache.as_ref().as_ref() {
                    None
                } else {
                    Some(IpcEvent::Metadata(service, new_metadata))
                }
            }
        })
        .boxed()
}

/// Emits when a player's volume moves off the cached one.
async fn volume_stream(entry: &MprisPlayerData) -> BoxedStream {
    let service = entry.service.clone();
    let volume = entry.volume;

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
        .boxed()
}

/// Emits when a player's playback state moves off the cached one.
async fn playback_stream(entry: &MprisPlayerData) -> BoxedStream {
    let service = entry.service.clone();
    let state = entry.state;

    entry
        .proxy
        .receive_playback_status_changed()
        .await
        .filter_map(move |signal| {
            let service = service.clone();

            async move {
                let new_state = signal.get().await.map_or_default(PlaybackStatus::from);

                if new_state == state {
                    None
                } else {
                    Some(IpcEvent::State(service, new_state))
                }
            }
        })
        .boxed()
}
