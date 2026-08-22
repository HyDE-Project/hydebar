//! The first reading of the network the bar takes from `NetworkManager`.

use log::debug;
use masterror::AppResult;

use super::NetworkDbus;
use crate::services::{
    bluetooth::BluetoothService,
    network::{LinkDetails, NetworkData}
};

/// Reads everything the network module draws on its first frame.
///
/// Airplane mode is not a state `NetworkManager` holds: it is read as radios
/// that are all off at once, so the wireless switch and the bluetooth soft
/// block are both asked before the answer is stated.
///
/// # Errors
///
/// Returns an error when the wireless devices, the active connections or the
/// reachable access points cannot be read.
pub(super) async fn initial_data(nm: &NetworkDbus<'_>) -> AppResult<NetworkData> {
    let bluetooth_soft_blocked = BluetoothService::check_rfkill_soft_block()
        .await
        .unwrap_or_default();

    let wifi_present = nm.wifi_device_present().await?;
    let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
    let airplane_mode = bluetooth_soft_blocked && !wifi_enabled;

    debug!("Wifi enabled: {wifi_enabled}, airplane mode: {airplane_mode}");

    let active_connections = nm.active_connections_info().await?;
    let wireless_access_points = nm.wireless_access_points().await?;
    let known_connections = nm
        .known_connections_internal(&wireless_access_points)
        .await?;

    debug!(
        "Active connections: {active_connections:?}, access points: {wireless_access_points:?}, \
         known: {known_connections:?}"
    );

    Ok(NetworkData {
        wifi_present,
        active_connections,
        wifi_enabled,
        airplane_mode,
        connectivity: nm.connectivity().await?,
        wireless_access_points,
        known_connections,
        scanning_nearby_wifi: false,
        link: LinkDetails::default(),
        last_error: None
    })
}
