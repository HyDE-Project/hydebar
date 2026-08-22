//! Turning the radios on and off, and putting a VPN up or down.

use std::collections::HashMap;

use log::debug;
use masterror::AppResult;
use tokio::process::Command;
use zbus::zvariant::{self, OwnedObjectPath};

use super::{NetworkDbus, proxies::WirelessDeviceProxy};
use crate::services::{bus::bus_failure, network::KnownConnection};

/// Blocks or unblocks every radio the bar can reach.
///
/// Bluetooth answers to `rfkill` and wireless to `NetworkManager`, and the two
/// are asked in that order so a failure to reach `rfkill` still leaves the
/// wireless switch in the state the user asked for.
///
/// # Errors
///
/// Returns an error when the wireless switch refuses to move.
pub(super) async fn set_airplane_mode(nm: &NetworkDbus<'_>, enable: bool) -> AppResult<()> {
    let blocked = Command::new("/usr/sbin/rfkill")
        .arg(if enable { "block" } else { "unblock" })
        .arg("bluetooth")
        .output()
        .await;

    if let Err(e) = blocked {
        debug!("Failed to set bluetooth rfkill: {e}");
    }

    let nm = NetworkDbus::new(nm.inner().connection()).await?;

    nm.set_wireless_enabled(!enable)
        .await
        .map_err(|e| bus_failure("Failed to set wireless enabled", &e))
}

/// Asks every wireless device to look for access points again.
///
/// # Errors
///
/// Returns an error when a device cannot be addressed or refuses the scan.
pub(super) async fn scan_nearby_wifi(nm: &NetworkDbus<'_>) -> AppResult<()> {
    for device_path in nm
        .wireless_access_points()
        .await?
        .iter()
        .map(|ap| ap.path.clone())
    {
        let device = WirelessDeviceProxy::builder(nm.inner().connection())
            .path(device_path)
            .map_err(|e| bus_failure("Failed to set WirelessDeviceProxy path", &e))?
            .build()
            .await
            .map_err(|e| bus_failure("Failed to build WirelessDeviceProxy", &e))?;

        device
            .request_scan(HashMap::new())
            .await
            .map_err(|e| bus_failure("Failed to request WiFi scan", &e))?;
    }

    Ok(())
}

/// Puts a VPN connection up or down and reports the list it belongs to.
///
/// # Errors
///
/// Returns an error when the connection refuses to activate or deactivate, or
/// when the refreshed list cannot be read.
pub(super) async fn set_vpn(
    nm: &NetworkDbus<'_>,
    connection: OwnedObjectPath,
    enable: bool
) -> AppResult<Vec<KnownConnection>> {
    if enable {
        debug!("Activating VPN: {connection:?}");

        let root = || zvariant::ObjectPath::from_static_str_unchecked("/").into();

        nm.activate_connection(connection, root(), root())
            .await
            .map_err(|e| bus_failure("Failed to activate VPN connection", &e))?;
    } else {
        debug!("Deactivating VPN: {connection:?}");

        nm.deactivate_connection(connection)
            .await
            .map_err(|e| bus_failure("Failed to deactivate VPN connection", &e))?;
    }

    let access_points = nm.wireless_access_points().await?;

    nm.known_connections_internal(&access_points).await
}
