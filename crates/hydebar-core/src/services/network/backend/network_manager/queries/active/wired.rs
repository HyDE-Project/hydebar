//! Description of an active wired connection.

use masterror::{AppError, AppResult};

use super::super::super::proxies::{ActiveConnectionProxy, DeviceProxy, WiredDeviceProxy};
use crate::services::network::ActiveConnectionInfo;

/// Reads the name and link speed of a wired connection.
pub(super) async fn describe(
    conn: &zbus::Connection,
    connection: &ActiveConnectionProxy<'_>,
    device: &DeviceProxy<'_>
) -> AppResult<ActiveConnectionInfo> {
    let wired_device = WiredDeviceProxy::builder(conn)
        .path(device.inner().path())
        .map_err(|e| AppError::internal(format!("Failed to set WiredDeviceProxy path: {e}")))?
        .build()
        .await
        .map_err(|e| AppError::internal(format!("Failed to build WiredDeviceProxy: {e}")))?;

    Ok(ActiveConnectionInfo::Wired {
        name:  connection.id().await.map_err(|e| {
            AppError::internal(format!("Failed to get wired connection ID: {e}"))
        })?,
        speed: wired_device
            .speed()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get wired device speed: {e}")))?
    })
}
