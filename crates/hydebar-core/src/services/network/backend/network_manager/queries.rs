//! Read only queries against the NetworkManager service.

use std::{collections::HashMap, ops::Deref};

use iced::futures::StreamExt;
use itertools::Itertools;
use log::warn;
use masterror::{AppError, AppResult};
use zbus::zvariant::{OwnedObjectPath, Value};

use super::{
    DeviceType, NetworkDbus, NetworkSettingsDbus,
    proxies::{
        AccessPointProxy, ActiveConnectionProxy, ConnectionSettingsProxy, DeviceProxy,
        WiredDeviceProxy, WirelessDeviceProxy
    }
};
use crate::services::network::{
    AccessPoint, ActiveConnectionInfo, ConnectivityState, DeviceState, KnownConnection, Vpn
};

impl<'a> NetworkDbus<'a> {
    pub async fn connectivity(&self) -> AppResult<ConnectivityState> {
        self.0
            .connectivity()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get connectivity state: {}", e)))
            .map(ConnectivityState::from)
    }

    pub async fn wifi_device_present(&self) -> AppResult<bool> {
        let devices = self
            .devices()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get devices: {}", e)))?;
        for d in devices {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .path(d)
                .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {}", e)))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {}", e)))?;

            if matches!(
                device.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                return Ok(true);
            }
        }

        Ok(false)
    }

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

    pub async fn known_connections_internal(
        &self,
        wireless_access_points: &[AccessPoint]
    ) -> AppResult<Vec<KnownConnection>> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;

        let known_connections = settings.know_connections().await?;

        let mut known_ssid = Vec::with_capacity(known_connections.len());
        let mut known_vpn = Vec::new();
        for c in known_connections {
            let cs = ConnectionSettingsProxy::builder(self.0.inner().connection())
                .path(c.clone())
                .map_err(|e| {
                    AppError::internal(format!(
                        "Failed to set ConnectionSettingsProxy path: {}",
                        e
                    ))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build ConnectionSettingsProxy: {}", e))
                })?;
            let Ok(s) = cs.get_settings().await else {
                warn!("Failed to get settings for connection {c}");
                continue;
            };

            let wifi = s.get("802-11-wireless");

            if wifi.is_some() {
                let ssid =
                    s.get("connection")
                        .and_then(|c| c.get("id"))
                        .map(|s| match s.deref() {
                            Value::Str(v) => v.to_string(),
                            _ => "".to_string()
                        });

                if let Some(cur_ssid) = ssid {
                    known_ssid.push(cur_ssid);
                }
            } else if s.contains_key("vpn") {
                let id = s
                    .get("connection")
                    .and_then(|c| c.get("id"))
                    .map(|v| match v.deref() {
                        Value::Str(v) => v.to_string(),
                        _ => "".to_string()
                    });

                if let Some(id) = id {
                    known_vpn.push(Vpn {
                        name: id, path: c
                    });
                }
            }
        }
        let known_connections: Vec<_> = wireless_access_points
            .iter()
            .filter_map(|a| {
                if known_ssid.contains(&a.ssid) {
                    Some(KnownConnection::AccessPoint(a.clone()))
                } else {
                    None
                }
            })
            .chain(known_vpn.into_iter().map(KnownConnection::Vpn))
            .collect();

        Ok(known_connections)
    }

    pub async fn wireless_devices(&self) -> AppResult<Vec<OwnedObjectPath>> {
        let devices = self
            .devices()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get devices: {}", e)))?;
        let mut wireless_devices = Vec::new();
        for d in devices {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .path(&d)
                .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {}", e)))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {}", e)))?;

            if matches!(
                device.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                wireless_devices.push(d);
            }
        }

        Ok(wireless_devices)
    }

    pub async fn wireless_access_points(&self) -> AppResult<Vec<AccessPoint>> {
        let wireless_devices = self.wireless_devices().await?;
        let wireless_access_point_futures: Vec<_> = wireless_devices
            .into_iter()
            .map(|path| async move {
                let device = DeviceProxy::builder(self.0.inner().connection())
                    .path(&path)
                    .map_err(|e| {
                        AppError::internal(format!("Failed to set DeviceProxy path: {}", e))
                    })?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to build DeviceProxy: {}", e))
                    })?;
                let wireless_device = WirelessDeviceProxy::builder(self.0.inner().connection())
                    .path(&path)
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to set WirelessDeviceProxy path: {}",
                            e
                        ))
                    })?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to build WirelessDeviceProxy: {}", e))
                    })?;
                wireless_device
                    .request_scan(HashMap::new())
                    .await
                    .map_err(|e| AppError::internal(format!("Failed to request scan: {}", e)))?;
                let mut scan_changed = wireless_device.receive_last_scan_changed().await;
                if let Some(t) = scan_changed.next().await
                    && let Ok(-1) = t.get().await
                {
                    return Ok(Default::default());
                }
                let access_points = wireless_device.get_access_points().await.map_err(|e| {
                    AppError::internal(format!("Failed to get access points: {}", e))
                })?;
                let state: DeviceState = device
                    .cached_state()
                    .unwrap_or_default()
                    .map(DeviceState::from)
                    .unwrap_or_else(|| DeviceState::Unknown);

                // Sort by strength and remove duplicates
                let mut aps = HashMap::<String, AccessPoint>::new();
                for ap in access_points {
                    let ap = AccessPointProxy::builder(self.0.inner().connection())
                        .path(ap)
                        .map_err(|e| {
                            AppError::internal(format!(
                                "Failed to set AccessPointProxy path: {}",
                                e
                            ))
                        })?
                        .build()
                        .await
                        .map_err(|e| {
                            AppError::internal(format!("Failed to build AccessPointProxy: {}", e))
                        })?;

                    let ssid = String::from_utf8_lossy(
                        &ap.ssid()
                            .await
                            .map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to get access point SSID: {}",
                                    e
                                ))
                            })?
                            .clone()
                    )
                    .into_owned();
                    let public = ap.flags().await.unwrap_or_default() == 0;
                    let strength = ap.strength().await.map_err(|e| {
                        AppError::internal(format!("Failed to get access point strength: {}", e))
                    })?;
                    if let Some(access_point) = aps.get(&ssid)
                        && access_point.strength > strength
                    {
                        continue;
                    }

                    aps.insert(
                        ssid.clone(),
                        AccessPoint {
                            ssid,
                            strength,
                            state,
                            public,
                            working: false,
                            path: ap.inner().path().clone().into(),
                            device_path: device.inner().path().clone().into()
                        }
                    );
                }

                let aps = aps
                    .into_values()
                    .sorted_by(|a, b| b.strength.cmp(&a.strength))
                    .collect();

                Ok(aps)
            })
            .collect();

        let mut wireless_access_points = Vec::with_capacity(wireless_access_point_futures.len());
        for f in wireless_access_point_futures {
            let mut access_points: AppResult<Vec<AccessPoint>> = f.await;
            if let Ok(access_points) = &mut access_points {
                wireless_access_points.append(access_points);
            }
        }

        wireless_access_points.sort_by(|a, b| b.strength.cmp(&a.strength));

        Ok(wireless_access_points)
    }
}
