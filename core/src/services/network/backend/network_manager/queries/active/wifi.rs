//! Description of an active wireless connection.

use masterror::{AppError, AppResult};

use super::super::super::proxies::{
    AccessPointProxy, ActiveConnectionProxy, DeviceProxy, WirelessDeviceProxy
};
use crate::services::network::ActiveConnectionInfo;

/// Reads the network name and signal strength of a wireless connection.
///
/// Returns `None` when the device is not associated with an access point.
pub(super) async fn describe(
    conn: &zbus::Connection,
    connection: &ActiveConnectionProxy<'_>,
    device: &DeviceProxy<'_>
) -> AppResult<Option<ActiveConnectionInfo>> {
    let wireless_device = WirelessDeviceProxy::builder(conn)
        .path(device.inner().path())
        .map_err(|e| AppError::internal(format!("Failed to set WirelessDeviceProxy path: {e}")))?
        .build()
        .await
        .map_err(|e| AppError::internal(format!("Failed to build WirelessDeviceProxy: {e}")))?;

    let Ok(access_point) = wireless_device.active_access_point().await else {
        return Ok(None);
    };

    let access_point = AccessPointProxy::builder(conn)
        .path(access_point)
        .map_err(|e| AppError::internal(format!("Failed to set AccessPointProxy path: {e}")))?
        .build()
        .await
        .map_err(|e| AppError::internal(format!("Failed to build AccessPointProxy: {e}")))?;

    let ssid = access_point
        .ssid()
        .await
        .map_err(|e| AppError::internal(format!("Failed to get access point SSID: {e}")))?;

    Ok(Some(ActiveConnectionInfo::WiFi {
        id:       connection
            .id()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get WiFi connection ID: {e}")))?,
        name:     String::from_utf8_lossy(&ssid).into_owned(),
        strength: access_point.strength().await.unwrap_or_default()
    }))
}
