//! Details of the currently active connections.

mod wifi;
mod wired;

use masterror::{AppError, AppResult};
use zbus::zvariant::OwnedObjectPath;

use super::super::{
    DeviceType, NetworkDbus,
    proxies::{ActiveConnectionProxy, DeviceProxy}
};
use crate::services::network::ActiveConnectionInfo;

impl NetworkDbus<'_> {
    /// Lists the object paths of the currently active connections.
    ///
    /// # Errors
    ///
    /// Returns an error when the daemon refuses the active connections query.
    pub async fn active_connections(&self) -> AppResult<Vec<OwnedObjectPath>> {
        self.0
            .active_connections()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get active connections: {e}")))
    }

    /// Describes every active connection with its device details.
    ///
    /// # Errors
    ///
    /// Returns an error when the active connections cannot be listed, a proxy
    /// for a connection or device cannot be built, or a connection refuses to
    /// report its details.
    pub async fn active_connections_info(&self) -> AppResult<Vec<ActiveConnectionInfo>> {
        let conn = self.0.inner().connection();
        let mut info = Vec::<ActiveConnectionInfo>::new();

        for path in self.active_connections().await? {
            let connection = ActiveConnectionProxy::builder(conn)
                .path(&path)
                .map_err(|e| {
                    AppError::internal(format!("Failed to set ActiveConnectionProxy path: {e}"))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build ActiveConnectionProxy: {e}"))
                })?;

            if connection.vpn().await.unwrap_or_default() {
                info.push(vpn_info(&connection, "VPN").await?);
                continue;
            }

            for device_path in connection.devices().await.unwrap_or_default() {
                let device = DeviceProxy::builder(conn)
                    .path(device_path)
                    .map_err(|e| {
                        AppError::internal(format!("Failed to set DeviceProxy path: {e}"))
                    })?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to build DeviceProxy for active connection: {e}"
                        ))
                    })?;

                match device.device_type().await.map(DeviceType::from).ok() {
                    Some(DeviceType::Ethernet) => {
                        info.push(wired::describe(conn, &connection, &device).await?);
                    }
                    Some(DeviceType::Wifi) => {
                        if let Some(entry) = wifi::describe(conn, &connection, &device).await? {
                            info.push(entry);
                        }
                    }
                    Some(DeviceType::WireGuard) => {
                        info.push(vpn_info(&connection, "WireGuard").await?);
                    }
                    _ => {}
                }
            }
        }

        info.sort_by_key(sort_key);

        Ok(info)
    }
}

/// Builds the entry describing a tunnelled connection.
async fn vpn_info(
    connection: &ActiveConnectionProxy<'_>,
    kind: &str
) -> AppResult<ActiveConnectionInfo> {
    Ok(ActiveConnectionInfo::Vpn {
        name:        connection
            .id()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get {kind} connection ID: {e}")))?,
        object_path: connection.inner().path().to_owned().into()
    })
}

/// Orders tunnels first, then wired links, then wireless networks.
fn sort_key(conn: &ActiveConnectionInfo) -> String {
    match conn {
        ActiveConnectionInfo::Vpn {
            name, ..
        } => format!("0{name}"),
        ActiveConnectionInfo::Wired {
            name, ..
        } => format!("1{name}"),
        ActiveConnectionInfo::WiFi {
            name, ..
        } => format!("2{name}")
    }
}
