//! Event stream assembled from iwd D-Bus signals.
//!
//! Four sources are merged into one: an adapter losing or gaining power, a
//! station changing state, a station starting or finishing a scan, and the
//! signal agent iwd calls when the link crosses one of the bands it was
//! registered with.
//!
//! What iwd could tell the bar and is not asked yet: interfaces appearing
//! and disappearing while the bar runs, so a wireless card plugged in
//! mid-session is not noticed until something else redraws the list; and the
//! authorisation agent, without which a network needing a password is joined
//! through the daemon rather than through the bar.

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
        let iwd = self;

        let mut wireless_enabled_changes = vec![];
        for adapter_proxy in self.adapters().await? {
            let stream = adapter_proxy
                .receive_powered_changed()
                .await
                .then({
                    move |p| async move {
                        let value = p.get().await.unwrap_or(false);
                        debug!("Adapter Powered changed: {value}");

                        let wifi_enabled = iwd.wireless_enabled().await.unwrap_or(false);
                        vec![NetworkEvent::WiFiEnabled(wifi_enabled)]
                    }
                })
                .boxed();
            wireless_enabled_changes.push(stream);
        }

        let stations = self.stations().await?;
        let mut connectivity_changes = vec![];
        let mut ap_s_kap_changes = vec![];
        let mut signal_level_updates = vec![];
        for station in stations {
            let cstream = station
                .receive_state_changed()
                .await
                .then({
                    move |p| async move {
                        let value = p.get().await.unwrap_or_default();
                        debug!("Station state changed: {value:?}");

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

                        let mut events = vec![NetworkEvent::KnownConnections(kcs)];

                        if is_scanning {
                            debug!("Scanning wifi");
                            events.push(NetworkEvent::ScanningNearbyWifi);
                            events.push(NetworkEvent::WirelessAccessPoint(aps));
                        } else {
                            debug!("Stopped scanning wifi");
                            events.push(NetworkEvent::WirelessDevice {
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

            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<i16>();
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

            let joined = station.clone();
            signal_level_updates.push(
                UnboundedReceiverStream::new(rx)
                    .filter_map(move |level| {
                        let joined = joined.clone();

                        async move {
                            let ssid = iwd.connected_network_name(&joined).await?;

                            debug!("Signal level on {ssid} changed: {level}");

                            Some(vec![NetworkEvent::Strength((
                                ssid,
                                strength_of_level(level)
                            ))])
                        }
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

        let events = select_all(vec![
            select_all(wireless_enabled_changes).boxed(),
            select_all(connectivity_changes).boxed(),
            select_all(ap_s_kap_changes).boxed(),
            select_all(signal_level_updates).boxed(),
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
