//! Wireless devices and the access points they can see.

mod scan;

use masterror::AppResult;
use zbus::zvariant::OwnedObjectPath;

use super::super::{DeviceType, NetworkDbus, proxies::DeviceProxy};
use crate::services::{bus::bus_failure, network::AccessPoint};

impl NetworkDbus<'_> {
    /// Lists the object paths of every wifi device.
    ///
    /// # Errors
    ///
    /// Returns an error when the devices cannot be listed or a device proxy
    /// cannot be built.
    pub async fn wireless_devices(&self) -> AppResult<Vec<OwnedObjectPath>> {
        let conn = self.0.inner().connection();
        let devices = self
            .devices()
            .await
            .map_err(|e| bus_failure("Failed to get devices", &e))?;

        let mut wireless_devices = Vec::new();
        for path in devices {
            let device = DeviceProxy::builder(conn)
                .path(&path)
                .map_err(|e| bus_failure("Failed to set DeviceProxy path", &e))?
                .build()
                .await
                .map_err(|e| bus_failure("Failed to build DeviceProxy", &e))?;

            if matches!(
                device.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                wireless_devices.push(path);
            }
        }

        Ok(wireless_devices)
    }

    /// Lists the access points every wifi device can currently see.
    ///
    /// # Errors
    ///
    /// Returns an error when the wireless devices cannot be listed.
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
