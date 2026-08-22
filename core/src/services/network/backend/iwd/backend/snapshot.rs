//! Full state snapshots read from the iwd daemon.

use iced::futures::future::join_all;
use log::debug;
use masterror::{AppError, AppResult};

use super::super::{IwdDbus, queries, station::StationProxy};
use crate::services::{
    bluetooth::BluetoothService,
    network::{
        AccessPoint, ConnectivityState, DeviceState, KnownConnection, LinkDetails, NetworkData
    }
};

/// Reads the complete network state the service starts from.
///
/// Airplane mode is not a daemon property: it is inferred from the rfkill
/// soft block on Bluetooth combined with the Wi-Fi switch being off.
///
/// # Errors
///
/// Returns an error when the daemon refuses any of the underlying queries.
pub(super) async fn initialize_data(iwd: &IwdDbus<'_>) -> AppResult<NetworkData> {
    let nm = iwd;

    let bluetooth_soft_blocked = BluetoothService::check_rfkill_soft_block()
        .await
        .unwrap_or_default();

    let wifi_present = nm.wifi_device_present().await?;

    let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
    debug!("Wifi enabled: {wifi_enabled}");

    let airplane_mode = bluetooth_soft_blocked && !wifi_enabled;
    debug!("Airplane mode: {airplane_mode}");

    let active_connections = nm.active_connections_info().await?;
    debug!("Active connections: {active_connections:?}");

    let wireless_access_points = nm.wireless_access_points().await?;
    debug!("Wireless access points: {wireless_access_points:?}");

    let known_connections = known_connections(iwd).await?;
    debug!("Known connections: {known_connections:?}");

    let is_scanning = join_all(iwd.stations().await?.iter().map(StationProxy::scanning))
        .await
        .into_iter()
        .filter_map(std::result::Result::ok)
        .any(|v| v);

    Ok(NetworkData {
        wifi_present,
        active_connections,
        wifi_enabled,
        airplane_mode,
        connectivity: nm
            .connectivity()
            .await?
            .into_iter()
            .map(ConnectivityState::from)
            .collect::<Vec<ConnectivityState>>()
            .into(),
        wireless_access_points,
        known_connections,
        scanning_nearby_wifi: is_scanning,
        link: LinkDetails::default(),
        last_error: None
    })
}

/// Lists the known (provisioned) SSIDs.
///
/// Each entry carries the state of the station that can reach it, so a
/// provisioned network the machine is currently on is drawn as connected
/// rather than as one more entry in the list.
///
/// # Errors
///
/// Returns an error when a network's name, device, or type cannot be read.
pub(super) async fn known_connections(iwd: &IwdDbus<'_>) -> AppResult<Vec<KnownConnection>> {
    let states = iwd.station_states().await?;
    let nets = iwd.reachable_networks().await?;
    let mut networks = Vec::new();
    for (n, s) in nets {
        if n.known_network().await.is_err() {
            continue;
        }
        let ssid = n
            .name()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get network name: {e}")))?;
        let path = n.inner().path().clone().into();
        let device_path = n
            .device()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get network device: {e}")))?
            .clone();
        let state = states
            .get(&device_path)
            .copied()
            .unwrap_or(DeviceState::Unknown);

        networks.push(KnownConnection::AccessPoint(AccessPoint {
            ssid,
            path,
            device_path,
            strength: queries::strength_from_rssi(s),
            state,
            public: n
                .type_()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get network type: {e}")))?
                == "open"
        }));
    }
    Ok(networks)
}
