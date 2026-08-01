//! Event stream assembled from `NetworkManager` D-Bus signals.
//!
//! The list of nearby access points is deliberately absent from it. The daemon
//! republishes `AccessPoints` whenever any neighbouring radio appears or fades,
//! which on a populated band is several times a second, and answering each one
//! meant re-reading every access point over the bus and repainting every
//! surface the bar owns for a list drawn nowhere but inside the network menu.
//! The bar asks for that list on its own clock instead, and only while the user
//! is looking at the menu.

use iced::futures::{
    Stream, StreamExt,
    stream::{BoxStream, select_all}
};
use log::debug;
use masterror::{AppError, AppResult};

use super::{
    NetworkBackend, NetworkDbus, NetworkSettingsDbus,
    proxies::{AccessPointProxy, DeviceProxy}
};
use crate::services::network::{ConnectivityState, DeviceState, NetworkEvent};

impl<'a> NetworkDbus<'a> {
    /// Assembles one stream carrying every network event the daemon signals.
    ///
    /// # Errors
    ///
    /// Returns an error when the settings service proxy cannot be created or
    /// when the devices needed for the per-device subscriptions cannot be
    /// listed.
    #[expect(
        clippy::too_many_lines,
        reason = "one subscription per NetworkManager signal is wired into a single merged stream; splitting would scatter the wiring"
    )]
    pub async fn subscribe_events(
        &'a self
    ) -> AppResult<impl Stream<Item = AppResult<NetworkEvent>> + 'a> {
        type EventStream<'s> = BoxStream<'s, AppResult<NetworkEvent>>;

        let conn = self.0.inner().connection();
        let settings = NetworkSettingsDbus::new(conn).await?;
        let mut streams: Vec<EventStream<'a>> = Vec::new();

        let wireless_enabled = self
            .clone()
            .receive_wireless_enabled_changed()
            .await
            .then(|signal| async move {
                let value = signal.get().await.map_err(|e| {
                    AppError::internal(format!("Failed to get wireless enabled state: {e}"))
                })?;

                debug!("WiFi enabled changed: {value}");
                Ok(NetworkEvent::WiFiEnabled(value))
            })
            .boxed();
        streams.push(wireless_enabled);

        let connectivity_changed = self
            .clone()
            .receive_connectivity_changed()
            .await
            .then(|signal| async move {
                let value = ConnectivityState::from(signal.get().await.map_err(|e| {
                    AppError::internal(format!("Failed to get connectivity state: {e}"))
                })?);

                debug!("Connectivity changed: {value:?}");
                Ok(NetworkEvent::Connectivity(value))
            })
            .boxed();
        streams.push(connectivity_changed);

        let active_connections_changes = self
            .clone()
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
            .boxed();
        streams.push(active_connections_changes);

        let devices = self.wireless_devices().await?;

        let wireless_devices_changed = self
            .clone()
            .receive_devices_changed()
            .await
            .then({
                let backend = self.clone();
                let devices = devices.clone();
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
        streams.push(wireless_devices_changed);

        let wireless_access_points = self.wireless_access_points().await?;

        let mut device_state_changes = Vec::with_capacity(wireless_access_points.len());
        for access_point in &wireless_access_points {
            let device_proxy = DeviceProxy::builder(conn)
                .path(access_point.device_path.clone())
                .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {e}")))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {e}")))?;

            let ssid = access_point.ssid.clone();
            device_state_changes.push(
                device_proxy
                    .receive_state_changed()
                    .await
                    .then({
                        let ssid = ssid.clone();
                        move |state| {
                            let ssid = ssid.clone();
                            async move {
                                let value =
                                    state.get().await.map(DeviceState::from).map_err(|e| {
                                        AppError::internal(format!(
                                            "Failed to get device state: {e}"
                                        ))
                                    })?;
                                if value == DeviceState::NeedAuth {
                                    debug!("Request password for ssid {ssid}");
                                    Ok(Some(NetworkEvent::RequestPasswordForSSID(ssid)))
                                } else {
                                    Ok(None)
                                }
                            }
                        }
                    })
                    .filter_map(|result| async move { result.transpose() })
                    .boxed()
            );
        }

        if !device_state_changes.is_empty() {
            let device_states = select_all(device_state_changes).boxed();
            streams.push(device_states);
        }

        let mut strength_changes_streams = Vec::with_capacity(wireless_access_points.len());
        for access_point in wireless_access_points {
            let ssid = access_point.ssid.clone();
            let proxy = AccessPointProxy::builder(conn)
                .path(access_point.path.clone())
                .map_err(|e| {
                    AppError::internal(format!("Failed to set AccessPointProxy path: {e}"))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build AccessPointProxy: {e}"))
                })?;

            strength_changes_streams.push(
                proxy
                    .receive_strength_changed()
                    .await
                    .then({
                        let ssid = ssid.clone();
                        move |signal| {
                            let ssid = ssid.clone();
                            async move {
                                let value = signal.get().await.map_err(|e| {
                                    AppError::internal(format!(
                                        "Failed to get signal strength: {e}"
                                    ))
                                })?;
                                debug!("Strength changed value: {ssid}, {value}");
                                Ok(NetworkEvent::Strength((ssid, value)))
                            }
                        }
                    })
                    .boxed()
            );
        }

        let strength_changes = select_all(strength_changes_streams).boxed();
        streams.push(strength_changes);

        let known_connections = settings
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
            .boxed();
        streams.push(known_connections);

        let events = select_all(streams);

        Ok(events)
    }
}
