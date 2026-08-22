//! Connectivity and device presence checks.

use masterror::AppResult;

use super::super::{DeviceType, NetworkDbus, proxies::DeviceProxy};
use crate::services::{bus::bus_failure, network::ConnectivityState};

impl NetworkDbus<'_> {
    /// Reads the daemon's overall connectivity state.
    ///
    /// # Errors
    ///
    /// Returns an error when the daemon refuses the connectivity query.
    pub async fn connectivity(&self) -> AppResult<ConnectivityState> {
        self.0
            .connectivity()
            .await
            .map_err(|e| bus_failure("Failed to get connectivity state", &e))
            .map(ConnectivityState::from)
    }

    /// Reports whether any wifi device is known to the daemon.
    ///
    /// # Errors
    ///
    /// Returns an error when the devices cannot be listed or a device proxy
    /// cannot be built.
    pub async fn wifi_device_present(&self) -> AppResult<bool> {
        let devices = self
            .devices()
            .await
            .map_err(|e| bus_failure("Failed to get devices", &e))?;
        for d in devices {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .path(d)
                .map_err(|e| bus_failure("Failed to set DeviceProxy path", &e))?
                .build()
                .await
                .map_err(|e| bus_failure("Failed to build DeviceProxy", &e))?;

            if matches!(
                device.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
