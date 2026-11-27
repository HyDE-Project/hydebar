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

use iced::futures::{
    Stream, StreamExt,
    stream::{BoxStream, select_all}
};
use log::debug;
use masterror::{AppError, AppResult};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use zbus::OwnedObjectPath;

use super::{
    IwdDbus, adapter::AdapterProxy, agents::SignalAgent, signal_agents::SignalAgents,
    station::StationProxy
};

mod bands;

use bands::strength_of_level;

use crate::services::{
    bus::bus_failure,
    network::{ConnectivityState, NetworkBackend, NetworkEvent}
};

/// One iwd signal source, already shaped into network events.
type EventStream<'a> = BoxStream<'a, Vec<NetworkEvent>>;

impl IwdDbus<'_> {
    /// Assembles one stream carrying every network event iwd signals.
    ///
    /// # Errors
    ///
    /// Returns an error when the adapters, devices or stations cannot be
    /// listed, or when registering the signal level agent fails.
    pub async fn subscribe_events(&self) -> AppResult<impl Stream<Item = Vec<NetworkEvent>>> {
        let mut kept = SignalAgents::new(self.inner().connection().clone());

        let mut wireless_enabled_changes = Vec::new();
        for adapter in self.adapters().await? {
            wireless_enabled_changes.push(self.power_stream(adapter).await);
        }

        let mut connectivity_changes = Vec::new();
        let mut scan_changes = Vec::new();
        let mut signal_level_updates = Vec::new();

        for station in self.stations().await? {
            connectivity_changes.push(self.connectivity_stream(&station).await);
            scan_changes.push(self.scan_stream(&station).await);
            signal_level_updates.push(self.watch_signal_level(&station, &mut kept).await?);
        }

        let events = select_all(vec![
            select_all(wireless_enabled_changes).boxed(),
            select_all(connectivity_changes).boxed(),
            select_all(scan_changes).boxed(),
            select_all(signal_level_updates).boxed(),
        ]);

        Ok(kept.guarding(events))
    }

    /// One adapter losing or gaining power.
    async fn power_stream<'a>(&'a self, adapter: AdapterProxy<'a>) -> EventStream<'a> {
        let iwd = self;

        adapter
            .receive_powered_changed()
            .await
            .then(move |p| async move {
                let value = p.get().await.unwrap_or(false);
                debug!("Adapter Powered changed: {value}");

                let wifi_enabled = iwd.wireless_enabled().await.unwrap_or(false);
                vec![NetworkEvent::WiFiEnabled(wifi_enabled)]
            })
            .boxed()
    }

    /// One station changing state.
    async fn connectivity_stream<'a>(&'a self, station: &StationProxy<'a>) -> EventStream<'a> {
        let iwd = self;
        let station = station.clone();

        station
            .receive_state_changed()
            .await
            .then(move |p| async move {
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
            })
            .boxed()
    }

    /// One station starting or finishing a scan.
    async fn scan_stream<'a>(&'a self, station: &StationProxy<'a>) -> EventStream<'a> {
        let iwd = self;
        let station = station.clone();

        station
            .receive_scanning_changed()
            .await
            .then(move |s| async move {
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
                        wifi_present:           iwd.wireless_enabled().await.unwrap_or(false),
                        wireless_access_points: aps
                    });
                }

                events
            })
            .boxed()
    }

    /// One station's link crossing a signal band, through a registered agent.
    async fn watch_signal_level<'a>(
        &'a self,
        station: &StationProxy<'a>,
        kept: &mut SignalAgents
    ) -> AppResult<EventStream<'a>> {
        let iwd = self;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<i16>();
        let agent = SignalAgent {
            tx
        };

        let agent_path = OwnedObjectPath::try_from(format!(
            "/com/hydebar/signalagent/{}",
            Uuid::new_v4().as_simple()
        ))
        .map_err(|e| AppError::internal(format!("Failed to create agent path: {e}")))?;

        self.inner()
            .connection()
            .object_server()
            .at(&agent_path, agent)
            .await
            .map_err(|e| bus_failure("Failed to register signal level agent", &e))?;

        let joined = station.clone();
        let levels = UnboundedReceiverStream::new(rx)
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
            .boxed();

        station
            .register_signal_level_agent(&agent_path, &[-40, -50, -60])
            .await
            .map_err(|e| {
                AppError::internal(format!(
                    "Failed to register signal level agent with station: {e}"
                ))
            })?;
        debug!("Registered signal level agent at {agent_path}");
        kept.keep(station.inner().path().to_owned().into(), agent_path);

        Ok(levels)
    }
}
