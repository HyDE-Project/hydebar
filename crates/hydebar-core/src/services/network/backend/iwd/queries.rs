//! Read only queries against the iwd daemon.

use masterror::{AppError, AppResult};

use super::{IwdDbus, device::DeviceProxy, network::NetworkProxy};
use crate::services::network::{AccessPoint, ActiveConnectionInfo, DeviceState};

/// Turns the raw RSSI iwd reports into the percentage the bar renders.
///
/// iwd hands back signal strength in centi-dBm, roughly 0 down to -10000,
/// while every consumer expects 0 to 100. The clamp keeps values outside
/// that window from wrapping when narrowed to `u8`.
#[expect(
    clippy::cast_sign_loss,
    reason = "the value is clamped to [0, 100] just before the cast, so it is never negative"
)]
pub(super) fn strength_from_rssi(s: i16) -> u8 {
    (s / 100 + 100).clamp(0, 100) as u8
}

impl IwdDbus<'_> {
    /// Get the state of all station interfaces
    ///
    /// # Errors
    ///
    /// Returns an error when the stations cannot be listed or a station
    /// refuses to report its state.
    pub async fn connectivity(&self) -> AppResult<Vec<String>> {
        let mut states = Vec::new();
        for s in self.stations().await? {
            let state = s
                .state()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get station state: {e}")))?;
            states.push(state);
        }
        Ok(states)
    }

    /// Return true if any device in station mode is present
    ///
    /// # Errors
    ///
    /// Returns an error when the wireless devices cannot be listed or a device
    /// refuses to report its powered state.
    pub async fn wifi_device_present(&self) -> AppResult<bool> {
        let devices = self.wireless_devices().await?;

        for d in devices {
            if d.powered().await.map_err(|e| {
                AppError::internal(format!("Failed to get device powered state: {e}"))
            })? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// List all networks currently connected (Connected = true)
    ///
    /// # Errors
    ///
    /// Returns an error when the reachable networks cannot be listed or a
    /// network refuses to report its connected state.
    pub async fn active_connections(&self) -> AppResult<Vec<(NetworkProxy, i16)>> {
        let mut networks = Vec::new();
        for (net, strength) in self.reachable_networks().await? {
            if net.connected().await.map_err(|e| {
                AppError::internal(format!("Failed to check network connected state: {e}"))
            })? {
                networks.push((net, strength));
            }
        }
        Ok(networks)
    }

    /// Detailed info on active connections
    ///
    /// # Errors
    ///
    /// Returns an error when the active connections cannot be listed or a
    /// network refuses to report its name.
    pub async fn active_connections_info(&self) -> AppResult<Vec<ActiveConnectionInfo>> {
        // INFO: probably way cleaner with a custom dbus object - SignalLevelAgent

        let nets = self.active_connections().await?;
        let mut info = Vec::new();
        for (net, s) in nets {
            let ssid = net
                .name()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get network name: {e}")))?;
            info.push(ActiveConnectionInfo::WiFi {
                id:       ssid.clone(),
                name:     ssid,
                strength: strength_from_rssi(s)
            });
        }
        Ok(info)
    }

    /// List all wireless (station-mode) devices
    ///
    /// # Errors
    ///
    /// Returns an error when the devices cannot be listed or a device refuses
    /// to report its mode.
    pub async fn wireless_devices(&self) -> AppResult<Vec<DeviceProxy>> {
        let devices = self.devices().await?;
        let mut devs = Vec::new();
        for d in devices {
            if d.mode()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get device mode: {e}")))?
                == "station"
            {
                devs.push(d);
            }
        }
        Ok(devs)
    }

    /// Scan and list available access points
    ///
    /// # Errors
    ///
    /// Returns an error when the reachable networks cannot be listed or a
    /// network refuses to report its name, type or device.
    pub async fn wireless_access_points(&self) -> AppResult<Vec<AccessPoint>> {
        let mut aps = Vec::new();
        {
            let nets = self.reachable_networks().await?;
            for (net, s) in nets {
                let ssid = net.name().await.map_err(|e| {
                    AppError::internal(format!("Failed to get network name: {e}"))
                })?;
                let public = net.type_().await.map_err(|e| {
                    AppError::internal(format!("Failed to get network type: {e}"))
                })? == "open";
                let path = net.inner().path().clone().into();
                let device_path = net
                    .device()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to get network device: {e}"))
                    })?
                    .clone();
                aps.push(AccessPoint {
                    ssid,
                    state: DeviceState::Unknown, // TODO:
                    strength: strength_from_rssi(s),
                    public,
                    working: false, // TODO:
                    path,
                    device_path
                });
            }
        }
        aps.sort_by_key(|ap| std::cmp::Reverse(ap.strength));
        Ok(aps)
    }

    /// Return true if any wireless device is powered.
    ///
    /// # Errors
    ///
    /// Returns an error when the wireless devices cannot be listed or a device
    /// refuses to report its powered state.
    pub async fn wireless_enabled(&self) -> AppResult<bool> {
        let devs = self.wireless_devices().await?;
        for d in devs {
            if d.powered().await.map_err(|e| {
                AppError::internal(format!("Failed to get device powered state: {e}"))
            })? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::strength_from_rssi;

    #[test]
    fn a_typical_rssi_maps_onto_the_middle_of_the_scale() {
        assert_eq!(strength_from_rssi(-4000), 60);
    }

    #[test]
    fn an_rssi_below_the_floor_clamps_to_zero() {
        assert_eq!(strength_from_rssi(-10500), 0);
    }

    #[test]
    fn a_zero_rssi_clamps_to_a_full_signal() {
        assert_eq!(strength_from_rssi(0), 100);
    }

    #[test]
    fn the_smallest_rssi_does_not_panic() {
        assert_eq!(strength_from_rssi(i16::MIN), 0);
    }
}
