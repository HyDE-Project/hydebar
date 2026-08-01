//! Wireless devices and the access points they can see.

mod scan;

use masterror::{AppError, AppResult};
use zbus::zvariant::OwnedObjectPath;

use super::super::{DeviceType, NetworkDbus, proxies::DeviceProxy};
use crate::services::network::AccessPoint;

impl NetworkDbus<'_> {
    pub async fn wireless_devices(&self) -> AppResult<Vec<OwnedObjectPath>> {
        let conn = self.0.inner().connection();
        let devices = self
            .devices()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get devices: {e}")))?;

        let mut wireless_devices = Vec::new();
        for path in devices {
            let device = DeviceProxy::builder(conn)
                .path(&path)
                .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {e}")))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {e}")))?;

            if matches!(
                device.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                wireless_devices.push(path);
            }
        }

        Ok(wireless_devices)
    }

    pub async fn wireless_access_points(&self) -> AppResult<Vec<AccessPoint>> {
        let conn = self.0.inner().connection();
        let mut all = Vec::new();

        for path in self.wireless_devices().await? {
            if let Ok(mut found) = scan::access_points_of(conn, &path).await {
                all.append(&mut found);
            }
        }

        all.sort_by_key(|ap| std::cmp::Reverse(ap.strength));

        Ok(all)
    }
}
