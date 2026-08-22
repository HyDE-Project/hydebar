//! Event stream assembled from iwd D-Bus signals.

use iced::futures::{Stream, StreamExt, stream::select_all};
use log::{debug, warn};
use masterror::{AppError, AppResult};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use zbus::zvariant::OwnedObjectPath;

use super::{IwdDbus, agents::SignalAgent};
use crate::services::{
    bus::bus_failure,
    network::{ConnectivityState, NetworkBackend, NetworkEvent}
};

impl IwdDbus<'_> {
    /// Assembles one stream carrying every network event iwd signals.
    ///
    /// # Errors
    ///
    /// Returns an error when the adapters, devices or stations cannot be
    /// listed, or when registering the signal level agent fails.
    #[expect(
        clippy::too_many_lines,
        reason = "one subscription per iwd signal is wired into a single merged stream; splitting would scatter the wiring"
    )]
    pub async fn subscribe_events(&self) -> AppResult<impl Stream<Item = Vec<NetworkEvent>>> {
        let _conn = self.inner().connection();
        let iwd = self;

        //self.register_psk_agent().await?;

        // --- WiFi Enabled ---
        let mut wireless_enabled_changes = vec![];
        for adapter_proxy in self.adapters().await? {
            let stream = adapter_proxy
                .receive_powered_changed()
                .await
                .then({
                    move |p| async move {
                        // Add move here
                        let value = p.get().await.unwrap_or(false);
                        debug!("Adapter Powered changed: {value}");
                        // We need to check *all* adapters to determine overall wifi state
                        let wifi_enabled = iwd.wireless_enabled().await.unwrap_or(false);
                        vec![NetworkEvent::WiFiEnabled(wifi_enabled)]
                    }
                })
                .boxed();
            wireless_enabled_changes.push(stream);
        }

        // connectivity, access points, strengths and known - all in one
        let stations = self.stations().await?;
        let mut connectivity_changes = vec![];
        let mut ap_s_kap_changes = vec![];
        let mut signal_level_updates = vec![];
        for station in stations {
            // this gets also triggered when connecting to new networks, so no need to
            // listen to network changes
            let cstream = station
                .receive_state_changed()
                .await
                .then({
                    move |p| async move {
                        let value = p.get().await.unwrap_or_default();
                        debug!("Station state changed: {value:?}");
                        // We need to check *all* stations to determine overall wifi state
                        vec![
                            NetworkEvent::Connectivity(
                                iwd.connectivity()
                                    .await
                                    .unwrap_or_default()
                                    .into_iter()
                                    .map(ConnectivityState::from)
                                    .collect::<Vec<ConnectivityState>>()
                                    .into()
                            ),
                            NetworkEvent::ActiveConnections(
                                iwd.active_connections_info().await.unwrap_or_default()
                            ),
                        ]
                    }
                })
                .boxed();
            connectivity_changes.push(cstream);

            let apstream = station
                .receive_scanning_changed()
                .await
                .then({
                    move |s| async move {
                        let is_scanning = s.get().await.unwrap_or(false);

                        let aps = iwd.wireless_access_points().await.unwrap_or_default();
                        let kcs = iwd.known_connections().await.unwrap_or_default();

                        let mut events = vec![
                            NetworkEvent::KnownConnections(kcs),
                            // TODO: Strength((String, u8)), <- responibility for the signal agent
                        ];
                        if is_scanning {
                            debug!("Scanning wifi");
                            events.push(NetworkEvent::ScanningNearbyWifi);
                            // to update list, if scanning stopped use device
                            events.push(NetworkEvent::WirelessAccessPoint(aps));
                        } else {
                            debug!("Stopped scanning wifi");
                            events.push(NetworkEvent::WirelessDevice {
                                // TODO: can we reasonably assume this is true here?
                                wifi_present:           iwd
                                    .wireless_enabled()
                                    .await
                                    .unwrap_or(false),
                                wireless_access_points: aps
                            });
                        }
                        events
                    }
                })
                .boxed();
            ap_s_kap_changes.push(apstream);

            // 2) channel
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<i16>();
            // 3) export agent
            let agent = SignalAgent {
                tx
            };

            let agent_path = OwnedObjectPath::try_from(format!(
                "/com/hydebar/signalagent/{}",
                Uuid::new_v4().as_simple()
            ))
            .map_err(|e| AppError::internal(format!("Failed to create agent path: {e}")))?;

            let _server = self
                .inner()
                .connection()
                .object_server()
                .at(&agent_path, agent)
                .await
                .map_err(|e| bus_failure("Failed to register signal level agent", &e))?;
            // 6) turn receiver into a Stream
            signal_level_updates.push(
                UnboundedReceiverStream::new(rx)
                    .filter_map(|level| async move {
                        debug!("Signal level changed: {level}");
                        // TODO: get current network name
                        Some(vec![NetworkEvent::Strength((
                            String::new(),
                            strength_of_level(level)
                        ))])
                    })
                    .boxed()
            );

            station
                .register_signal_level_agent(&agent_path, &[-40, -50, -60])
                .await
                .map_err(|e| {
                    AppError::internal(format!(
                        "Failed to register signal level agent with station: {e}"
                    ))
                })?;
            warn!("Registered signal level agent at {agent_path}");
        }

        // TODO: probably would need to listen to interfaces registered and unregistered
        //let devices = nm.wireless_devices().await.unwrap_or_default();

        //let wireless_devices_changed = nm
        //    .receive_devices_changed()
        //    .await
        //    .filter_map({
        //        let conn = conn.clone();
        //        let devices = devices.clone();
        //        move |_| {
        //            let conn = conn.clone();
        //            let devices = devices.clone();
        //            async move {
        //                let nm = NetworkDbus::new(&conn).await.unwrap();

        //                let current_devices =
        // nm.wireless_devices().await.unwrap_or_default();                if
        // current_devices != devices {                    let wifi_present =
        // nm.wifi_device_present().await.unwrap_or_default();
        // let wireless_access_points =
        // nm.wireless_access_points().await.unwrap_or_default();

        //                    debug!(
        //                        "Wireless device changed: wifi present {:?},
        // wireless_access_points {:?}",                        wifi_present,
        // wireless_access_points,                    );
        //                    Some(NetworkEvent::WirelessDevice {
        //                        wifi_present,
        //                        wireless_access_points,
        //                    })
        //                } else {
        //                    None
        //                }
        //            }
        //        }
        //    })
        //    .boxed();

        //TODO: likely need to register an auth agent and wait for it here, same goes
        // for network configuration etc - these all are agents registered with
        // IWD - and represent device states

        //// When devices list change I need to update the wireless device state changes
        //let wireless_ac = nm.wireless_access_points().await?;

        //let mut device_state_changes = Vec::with_capacity(wireless_ac.len());
        //for ac in wireless_ac.iter() {
        //    let dp = DeviceProxy::builder(conn)
        //        .path(ac.device_path.clone())?
        //        .build()
        //        .await?;

        //    device_state_changes.push(
        //        dp.receive_state_changed()
        //            .await
        //            .filter_map(|val| async move {
        //                let val = val.get().await;
        //                let val = val.map(DeviceState::from).unwrap_or_default();

        //                if val == DeviceState::NeedAuth {
        //                    Some(val)
        //                } else {
        //                    None
        //                }
        //            })
        //            .map(|_| {
        //                let ssid = ac.ssid.clone();

        //                debug!("Request password for ssid {}", ssid);
        //                NetworkEvent::RequestPasswordForSSID(ssid)
        //            }),
        //    );
        //}

        //let access_points = select_all(ac_changes).boxed();

        let events = select_all(vec![
            select_all(wireless_enabled_changes).boxed(),
            select_all(connectivity_changes).boxed(),
            select_all(ap_s_kap_changes).boxed(),
            select_all(signal_level_updates).boxed(),
            // TODO: add a future that waits for 10s to poll certain information like the signal
            // strength change
        ]);

        Ok(events)
    }
}

/// Maps the bucket the signal agent reports onto a percentage.
///
/// The agent is registered with three thresholds, so iwd answers with the
/// index of the band the signal fell into — not a percentage. Rendering the
/// index directly showed the weakest icon whatever the actual signal.
const fn strength_of_level(level: i16) -> u8 {
    match level {
        0 => 100,
        1 => 75,
        2 => 50,
        _ => 25
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::strength_of_level;

    #[test]
    fn the_strongest_band_maps_to_a_full_signal() {
        assert_eq!(strength_of_level(0), 100);
    }

    #[test]
    fn the_second_band_maps_to_three_quarters() {
        assert_eq!(strength_of_level(1), 75);
    }

    #[test]
    fn the_third_band_maps_to_half() {
        assert_eq!(strength_of_level(2), 50);
    }

    #[test]
    fn any_other_band_maps_to_a_quarter() {
        assert_eq!(strength_of_level(3), 25);
        assert_eq!(strength_of_level(-1), 25);
        assert_eq!(strength_of_level(i16::MAX), 25);
        assert_eq!(strength_of_level(i16::MIN), 25);
    }
}
