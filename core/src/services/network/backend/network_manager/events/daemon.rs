//! Daemon-level signal subscriptions: radio state, connectivity, connections.

use iced::futures::StreamExt;
use log::debug;
use masterror::{AppError, AppResult};

use super::{
    EventStream,
    super::{NetworkBackend, NetworkDbus, NetworkSettingsDbus}
};
use crate::services::network::{ConnectivityState, NetworkEvent};

impl<'a> NetworkDbus<'a> {
    /// Stream of Wi-Fi radio enablement changes.
    pub(super) async fn wireless_enabled_events(&'a self) -> EventStream<'a> {
        self.clone()
            .receive_wireless_enabled_changed()
            .await
            .then(|signal| async move {
                let value = signal.get().await.map_err(|e| {
                    AppError::internal(format!("Failed to get wireless enabled state: {e}"))
                })?;

                debug!("WiFi enabled changed: {value}");
                Ok(NetworkEvent::WiFiEnabled(value))
            })
            .boxed()
    }

    /// Stream of connectivity state changes.
    pub(super) async fn connectivity_events(&'a self) -> EventStream<'a> {
        self.clone()
            .receive_connectivity_changed()
            .await
            .then(|signal| async move {
                let value = ConnectivityState::from(signal.get().await.map_err(|e| {
                    AppError::internal(format!("Failed to get connectivity state: {e}"))
                })?);

                debug!("Connectivity changed: {value:?}");
                Ok(NetworkEvent::Connectivity(value))
            })
            .boxed()
    }

    /// Stream re-reading the active connections whenever their set changes.
    pub(super) async fn active_connection_events(&'a self) -> EventStream<'a> {
        self.clone()
            .receive_active_connections_changed()
            .await
            .then({
                let backend = self.clone();
                move |_| {
                    let backend = backend.clone();
                    async move {
                        let value = backend.active_connections_info().await?;

                        debug!("Active connections changed: {value:?}");
                        Ok(NetworkEvent::ActiveConnections(value))
                    }
                }
            })
            .boxed()
    }

    /// Stream announcing a changed wireless device roster.
    ///
    /// # Errors
    ///
    /// Returns an error when the current wireless devices cannot be listed.
    pub(super) async fn device_roster_events(&'a self) -> AppResult<EventStream<'a>> {
        let devices = self.wireless_devices().await?;

        let stream = self
            .clone()
            .receive_devices_changed()
            .await
            .then({
                let backend = self.clone();
                move |_| {
                    let backend = backend.clone();
                    let devices = devices.clone();
                    async move {
                        let current_devices = backend.wireless_devices().await?;
                        if current_devices == devices {
                            Ok(None)
                        } else {
                            let wifi_present = backend.wifi_device_present().await?;
                            let wireless_access_points =
                                backend.wireless_access_points().await?;

                            debug!(
                                "Wireless device changed: wifi present {wifi_present:?}, wireless_access_points {wireless_access_points:?}",
                            );
                            Ok(Some(NetworkEvent::WirelessDevice {
                                wifi_present,
                                wireless_access_points,
                            }))
                        }
                    }
                }
            })
            .filter_map(|result| async move { result.transpose() })
            .boxed();

        Ok(stream)
    }

    /// Stream re-reading the known connections whenever the settings change.
    pub(super) async fn known_connection_events(
        &'a self,
        settings: NetworkSettingsDbus<'a>
    ) -> EventStream<'a> {
        settings
            .clone()
            .receive_connections_changed()
            .await
            .then({
                let backend = self.clone();
                move |_| {
                    let backend = backend.clone();
                    async move {
                        let known_connections = backend.known_connections().await?;

                        debug!("Known connections changed");
                        Ok(NetworkEvent::KnownConnections(known_connections))
                    }
                }
            })
            .boxed()
    }
}
