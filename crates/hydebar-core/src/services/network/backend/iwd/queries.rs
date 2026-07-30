//! Read only queries against the iwd daemon.

use masterror::{AppError, AppResult};

use super::{IwdDbus, device::DeviceProxy, network::NetworkProxy};
use crate::services::network::{AccessPoint, ActiveConnectionInfo, DeviceState};

impl IwdDbus<'_> {
    /// Get the state of all station interfaces
    pub async fn connectivity(&self) -> AppResult<Vec<String>> {
        let mut states = Vec::new();
        for s in self.stations().await? {
            let state = s
                .state()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get station state: {}", e)))?;
            states.push(state);
        }
        Ok(states)
    }

    /// Return true if any device in station mode is present
    pub async fn wifi_device_present(&self) -> AppResult<bool> {
        let devices = self.wireless_devices().await?;

        for d in devices {
            if d.powered().await.map_err(|e| {
                AppError::internal(format!("Failed to get device powered state: {}", e))
            })? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// List all networks currently connected (Connected = true)
    pub async fn active_connections(&self) -> AppResult<Vec<(NetworkProxy, i16)>> {
        let mut networks = Vec::new();
        for (net, strength) in self.reachable_networks().await? {
            if net.connected().await.map_err(|e| {
                AppError::internal(format!("Failed to check network connected state: {}", e))
            })? {
                networks.push((net, strength));
            }
        }
        Ok(networks)
    }

    /// Detailed info on active connections
    pub async fn active_connections_info(&self) -> AppResult<Vec<ActiveConnectionInfo>> {
        // INFO: probably way cleaner with a custom dbus object - SignalLevelAgent

        let nets = self.active_connections().await?;
        let mut info = Vec::new();
        for (net, s) in nets {
            let ssid = net
                .name()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get network name: {}", e)))?;
            // strength not directly on Network; placeholder 0
            info.push(ActiveConnectionInfo::WiFi {
                id:       ssid.clone(),
                name:     ssid,
                strength: (s / 100 + 100).clamp(0, 100) as u8
            });
        }
        Ok(info)
    }

    /// List all wireless (station-mode) devices
    pub async fn wireless_devices(&self) -> AppResult<Vec<DeviceProxy>> {
        let devices = self.devices().await?;
        let mut devs = Vec::new();
        for d in devices {
            if d.mode()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get device mode: {}", e)))?
                == "station"
            {
                devs.push(d);
            }
        }
        Ok(devs)
    }

    /// Scan and list available access points
    pub async fn wireless_access_points(&self) -> AppResult<Vec<AccessPoint>> {
        let mut aps = Vec::new();
        {
            let nets = self.reachable_networks().await?;
            for (net, s) in nets {
                let ssid = net.name().await.map_err(|e| {
                    AppError::internal(format!("Failed to get network name: {}", e))
                })?;
                let public = net.type_().await.map_err(|e| {
                    AppError::internal(format!("Failed to get network type: {}", e))
                })? == "open";
                let path = net.inner().path().clone().into();
                let device_path = net
                    .device()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to get network device: {}", e))
                    })?
                    .clone();
                aps.push(AccessPoint {
                    ssid,
                    state: DeviceState::Unknown, // TODO:
                    // _s is between 0 and -10000
                    // should be between 0 and 100
                    strength: ((s / 100) + 100).clamp(0, 100) as u8,
                    public,
                    working: false, // TODO:
                    path,
                    device_path
                });
            }
        }
        aps.sort_by(|a, b| b.strength.cmp(&a.strength));
        Ok(aps)
    }

    pub async fn wireless_enabled(&self) -> AppResult<bool> {
        let devs = self.wireless_devices().await?;
        for d in devs {
            if d.powered().await.map_err(|e| {
                AppError::internal(format!("Failed to get device powered state: {}", e))
            })? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
