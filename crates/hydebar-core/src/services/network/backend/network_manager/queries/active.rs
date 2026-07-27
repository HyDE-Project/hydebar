//! Details of the currently active connections.

use masterror::{AppError, AppResult};
use zbus::zvariant::OwnedObjectPath;

use super::super::{
    DeviceType, NetworkDbus,
    proxies::{
        AccessPointProxy, ActiveConnectionProxy, DeviceProxy, WiredDeviceProxy,
        WirelessDeviceProxy
    }
};
use crate::services::network::ActiveConnectionInfo;

impl<'a> NetworkDbus<'a> {
    pub async fn active_connections(&self) -> AppResult<Vec<OwnedObjectPath>> {
        let connections =
            self.0.active_connections().await.map_err(|e| {
                AppError::internal(format!("Failed to get active connections: {}", e))
            })?;

        Ok(connections)
    }

    pub async fn active_connections_info(&self) -> AppResult<Vec<ActiveConnectionInfo>> {
        let active_connections = self.active_connections().await?;
        let mut ac_proxies: Vec<ActiveConnectionProxy> =
            Vec::with_capacity(active_connections.len());
        for active_connection in &active_connections {
            let active_connection = ActiveConnectionProxy::builder(self.0.inner().connection())
                .path(active_connection)
                .map_err(|e| {
                    AppError::internal(format!("Failed to set ActiveConnectionProxy path: {}", e))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build ActiveConnectionProxy: {}", e))
                })?;
            ac_proxies.push(active_connection);
        }

        let mut info = Vec::<ActiveConnectionInfo>::with_capacity(active_connections.len());
        for connection in ac_proxies {
            if connection.vpn().await.unwrap_or_default() {
                info.push(ActiveConnectionInfo::Vpn {
                    name:        connection.id().await.map_err(|e| {
                        AppError::internal(format!("Failed to get VPN connection ID: {}", e))
                    })?,
                    object_path: connection.inner().path().to_owned().into()
                });
                continue;
            }
            for device in connection.devices().await.unwrap_or_default() {
                let device = DeviceProxy::builder(self.0.inner().connection())
                    .path(device)
                    .map_err(|e| {
                        AppError::internal(format!("Failed to set DeviceProxy path: {}", e))
                    })?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to build DeviceProxy for active connection: {}",
                            e
                        ))
                    })?;

                match device.device_type().await.map(DeviceType::from).ok() {
                    Some(DeviceType::Ethernet) => {
                        let wired_device = WiredDeviceProxy::builder(self.0.inner().connection())
                            .path(device.inner().path())
                            .map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to set WiredDeviceProxy path: {}",
                                    e
                                ))
                            })?
                            .build()
                            .await
                            .map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to build WiredDeviceProxy: {}",
                                    e
                                ))
                            })?;

                        info.push(ActiveConnectionInfo::Wired {
                            name:  connection.id().await.map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to get wired connection ID: {}",
                                    e
                                ))
                            })?,
                            speed: wired_device.speed().await.map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to get wired device speed: {}",
                                    e
                                ))
                            })?
                        });
                    }
                    Some(DeviceType::Wifi) => {
                        let wireless_device =
                            WirelessDeviceProxy::builder(self.0.inner().connection())
                                .path(device.inner().path())
                                .map_err(|e| {
                                    AppError::internal(format!(
                                        "Failed to set WirelessDeviceProxy path: {}",
                                        e
                                    ))
                                })?
                                .build()
                                .await
                                .map_err(|e| {
                                    AppError::internal(format!(
                                        "Failed to build WirelessDeviceProxy: {}",
                                        e
                                    ))
                                })?;

                        if let Ok(access_point) = wireless_device.active_access_point().await {
                            let access_point =
                                AccessPointProxy::builder(self.0.inner().connection())
                                    .path(access_point)
                                    .map_err(|e| {
                                        AppError::internal(format!(
                                            "Failed to set AccessPointProxy path: {}",
                                            e
                                        ))
                                    })?
                                    .build()
                                    .await
                                    .map_err(|e| {
                                        AppError::internal(format!(
                                            "Failed to build AccessPointProxy: {}",
                                            e
                                        ))
                                    })?;

                            info.push(ActiveConnectionInfo::WiFi {
                                id:       connection.id().await.map_err(|e| {
                                    AppError::internal(format!(
                                        "Failed to get WiFi connection ID: {}",
                                        e
                                    ))
                                })?,
                                name:     String::from_utf8_lossy(
                                    &access_point.ssid().await.map_err(|e| {
                                        AppError::internal(format!(
                                            "Failed to get access point SSID: {}",
                                            e
                                        ))
                                    })?
                                )
                                .into_owned(),
                                strength: access_point.strength().await.unwrap_or_default()
                            });
                        }
                    }
                    Some(DeviceType::WireGuard) => {
                        info.push(ActiveConnectionInfo::Vpn {
                            name:        connection.id().await.map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to get WireGuard connection ID: {}",
                                    e
                                ))
                            })?,
                            object_path: connection.inner().path().to_owned().into()
                        });
                    }
                    _ => {}
                }
            }
        }

        info.sort_by(|a, b| {
            let helper = |conn: &ActiveConnectionInfo| match conn {
                ActiveConnectionInfo::Vpn {
                    name, ..
                } => format!("0{name}"),
                ActiveConnectionInfo::Wired {
                    name, ..
                } => format!("1{name}"),
                ActiveConnectionInfo::WiFi {
                    name, ..
                } => format!("2{name}")
            };
            helper(a).cmp(&helper(b))
        });

        Ok(info)
    }
}
