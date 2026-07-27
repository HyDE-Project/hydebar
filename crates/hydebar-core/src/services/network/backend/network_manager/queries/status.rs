//! Connectivity and device presence checks.

use masterror::{AppError, AppResult};

use super::super::{DeviceType, NetworkDbus, proxies::DeviceProxy};
use crate::services::network::ConnectivityState;

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
}
