//! Wireless devices and the access points they can see.

use std::collections::HashMap;

use iced::futures::StreamExt;
use itertools::Itertools;
use masterror::{AppError, AppResult};
use zbus::zvariant::OwnedObjectPath;

use super::super::{
    DeviceType, NetworkDbus,
    proxies::{AccessPointProxy, DeviceProxy, WirelessDeviceProxy}
};
use crate::services::network::{AccessPoint, DeviceState};

impl<'a> NetworkDbus<'a> {
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
