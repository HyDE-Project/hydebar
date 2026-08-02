//! Discovery of MPRIS player services on the session bus.

use futures::future::join_all;
use masterror::{AppError, AppResult};
use zbus::{Connection, fdo::DBusProxy};

use crate::services::mpris::{
    data::{MprisPlayerData, MprisPlayerMetadata, PlaybackStatus},
    dbus::MprisPlayerProxy
};

/// Prefix applied to all MPRIS-compliant player service names on the session
/// bus.
pub const MPRIS_PLAYER_SERVICE_PREFIX: &str = "org.mpris.MediaPlayer2.";

/// Returns `true` when `name` references an MPRIS player service.
pub fn is_mpris_service(name: &str) -> bool {
    name.starts_with(MPRIS_PLAYER_SERVICE_PREFIX)
}

/// Fetches all available MPRIS players on the provided D-Bus `conn`.
pub async fn collect_players(conn: &Connection) -> AppResult<Vec<MprisPlayerData>> {
    let names = list_mpris_service_names(conn).await?;
    Ok(fetch_players(conn, &names).await)
}

async fn list_mpris_service_names(conn: &Connection) -> AppResult<Vec<String>> {
    let dbus = DBusProxy::new(conn)
        .await
        .map_err(|e| AppError::internal(format!("Failed to create DBusProxy: {e}")))?;
    let names = dbus
        .list_names()
        .await
        .map_err(|e| AppError::internal(format!("failed to list D-Bus names: {e}")))?
        .iter()
        .filter(|name| is_mpris_service(name))
        .map(ToString::to_string)
        .collect();

    Ok(names)
}

/// Retrieves `MprisPlayerData` entries for each service in `names`.
pub async fn fetch_players(conn: &Connection, names: &[String]) -> Vec<MprisPlayerData> {
    join_all(names.iter().map(|service| async {
        match MprisPlayerProxy::new(conn, service.clone()).await {
            Ok(proxy) => {
                let metadata = proxy.metadata().await.map(MprisPlayerMetadata::from).ok();
                let volume = proxy.volume().await.map(|value| value * 100.0).ok();
                let state = proxy.playback_status().await.map(PlaybackStatus::from);

                state.ok().map(|state| MprisPlayerData {
                    service: service.clone(),
                    metadata,
                    volume,
                    state,
                    proxy
                })
            }
            Err(_) => None
        }
    }))
    .await
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mpris_service_prefix() {
        assert!(is_mpris_service("org.mpris.MediaPlayer2.foo"));
        assert!(!is_mpris_service("org.freedesktop.DBus"));
    }
}
