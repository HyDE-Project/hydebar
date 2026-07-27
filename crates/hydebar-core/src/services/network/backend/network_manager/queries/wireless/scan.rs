//! Access point scan of a single wireless device.

use std::collections::HashMap;

use iced::futures::StreamExt;
use itertools::Itertools;
use masterror::{AppError, AppResult};
use zbus::zvariant::OwnedObjectPath;

use super::super::super::proxies::{AccessPointProxy, DeviceProxy, WirelessDeviceProxy};
use crate::services::network::{AccessPoint, DeviceState};

/// Scans one wireless device and returns the access points it can see,
/// strongest first and without duplicate network names.
pub(super) async fn access_points_of(
    conn: &zbus::Connection,
    path: &OwnedObjectPath
) -> AppResult<Vec<AccessPoint>> {
    let device = DeviceProxy::builder(conn)
        .path(path)
        .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {}", e)))?
        .build()
        .await
        .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {}", e)))?;

    let wireless_device = WirelessDeviceProxy::builder(conn)
        .path(path)
        .map_err(|e| AppError::internal(format!("Failed to set WirelessDeviceProxy path: {}", e)))?
        .build()
        .await
        .map_err(|e| AppError::internal(format!("Failed to build WirelessDeviceProxy: {}", e)))?;

    wireless_device
        .request_scan(HashMap::new())
        .await
        .map_err(|e| AppError::internal(format!("Failed to request scan: {}", e)))?;

    let mut scan_changed = wireless_device.receive_last_scan_changed().await;
    if let Some(t) = scan_changed.next().await
        && let Ok(-1) = t.get().await
    {
        return Ok(Vec::new());
    }

    let access_points = wireless_device
        .get_access_points()
        .await
        .map_err(|e| AppError::internal(format!("Failed to get access points: {}", e)))?;

    let state = device
        .cached_state()
        .unwrap_or_default()
        .map(DeviceState::from)
        .unwrap_or(DeviceState::Unknown);

    let mut strongest = HashMap::<String, AccessPoint>::new();
    for ap_path in access_points {
        let entry = read_access_point(conn, ap_path, &device, state).await?;

        if let Some(known) = strongest.get(&entry.ssid)
            && known.strength > entry.strength
        {
            continue;
        }

        strongest.insert(entry.ssid.clone(), entry);
    }

    Ok(strongest
        .into_values()
        .sorted_by(|a, b| b.strength.cmp(&a.strength))
        .collect())
}

/// Reads the properties of a single access point.
async fn read_access_point(
    conn: &zbus::Connection,
    path: OwnedObjectPath,
    device: &DeviceProxy<'_>,
    state: DeviceState
) -> AppResult<AccessPoint> {
    let ap = AccessPointProxy::builder(conn)
        .path(path)
        .map_err(|e| AppError::internal(format!("Failed to set AccessPointProxy path: {}", e)))?
        .build()
        .await
        .map_err(|e| AppError::internal(format!("Failed to build AccessPointProxy: {}", e)))?;

    let ssid = ap
        .ssid()
        .await
        .map_err(|e| AppError::internal(format!("Failed to get access point SSID: {}", e)))?;
    let strength = ap
        .strength()
        .await
        .map_err(|e| AppError::internal(format!("Failed to get access point strength: {}", e)))?;

    Ok(AccessPoint {
        ssid: String::from_utf8_lossy(&ssid).into_owned(),
        strength,
        state,
        public: ap.flags().await.unwrap_or_default() == 0,
        working: false,
        path: ap.inner().path().clone().into(),
        device_path: device.inner().path().clone().into()
    })
}
