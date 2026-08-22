//! Description of an active wired connection.

use masterror::AppResult;

use super::super::super::proxies::{ActiveConnectionProxy, DeviceProxy, WiredDeviceProxy};
use crate::services::{bus::bus_failure, network::ActiveConnectionInfo};

/// Reads the name and link speed of a wired connection.
pub(super) async fn describe(
    conn: &zbus::Connection,
    connection: &ActiveConnectionProxy<'_>,
    device: &DeviceProxy<'_>
) -> AppResult<ActiveConnectionInfo> {
    let wired_device = WiredDeviceProxy::builder(conn)
        .path(device.inner().path())
        .map_err(|e| bus_failure("Failed to set WiredDeviceProxy path", &e))?
        .build()
        .await
        .map_err(|e| bus_failure("Failed to build WiredDeviceProxy", &e))?;

    Ok(ActiveConnectionInfo::Wired {
        name:  connection
            .id()
            .await
            .map_err(|e| bus_failure("Failed to get wired connection ID", &e))?,
        speed: wired_device
            .speed()
            .await
            .map_err(|e| bus_failure("Failed to get wired device speed", &e))?
    })
}
