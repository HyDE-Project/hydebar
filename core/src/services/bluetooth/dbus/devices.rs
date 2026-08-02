//! Device enumeration and connection handling over bluez.

use masterror::{AppError, AppResult};
use zbus::zvariant::OwnedObjectPath;

use super::{
    BluetoothDbus,
    proxies::{BatteryProxy, DeviceProxy}
};
use crate::services::bluetooth::BluetoothDevice;

impl BluetoothDbus<'_> {
    pub async fn devices(&self) -> AppResult<Vec<BluetoothDevice>> {
        let devices_proxy = self
            .bluez
            .get_managed_objects()
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to get managed objects for devices: {e}"))
            })?
            .into_iter()
            .filter_map(|(key, item)| {
                if item.contains_key("org.bluez.Device1") {
                    Some((key, item.contains_key("org.bluez.Battery1")))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut devices = Vec::new();
        for (device_path, has_battery) in devices_proxy {
            let device = DeviceProxy::builder(self.bluez.inner().connection())
                .path(device_path.clone())
                .map_err(|e| AppError::internal(format!("Failed to set device path: {e}")))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {e}")))?;

            let name = device
                .alias()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get device alias: {e}")))?;
            let connected = device.connected().await.map_err(|e| {
                AppError::internal(format!("Failed to get device connected state: {e}"))
            })?;
            let paired = device.paired().await.unwrap_or(false);

            if paired {
                let battery = if connected && has_battery {
                    let battery_proxy = BatteryProxy::builder(self.bluez.inner().connection())
                        .path(&device_path)
                        .map_err(|e| {
                            AppError::internal(format!("Failed to set battery path: {e}"))
                        })?
                        .build()
                        .await
                        .map_err(|e| {
                            AppError::internal(format!("Failed to build BatteryProxy: {e}"))
                        })?;

                    Some(battery_proxy.percentage().await.map_err(|e| {
                        AppError::internal(format!("Failed to get battery percentage: {e}"))
                    })?)
                } else {
                    None
                };

                devices.push(BluetoothDevice {
                    name,
                    battery,
                    path: device_path,
                    connected
                });
            }
        }

        Ok(devices)
    }

    pub async fn connect_device(&self, device_path: &OwnedObjectPath) -> AppResult<()> {
        let device = DeviceProxy::builder(self.bluez.inner().connection())
            .path(device_path)
            .map_err(|e| {
                AppError::internal(format!("Failed to set device path for connect: {e}"))
            })?
            .build()
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to build DeviceProxy for connect: {e}"))
            })?;

        device
            .connect()
            .await
            .map_err(|e| AppError::internal(format!("Failed to connect device: {e}")))?;
        Ok(())
    }

    pub async fn disconnect_device(&self, device_path: &OwnedObjectPath) -> AppResult<()> {
        let device = DeviceProxy::builder(self.bluez.inner().connection())
            .path(device_path)
            .map_err(|e| {
                AppError::internal(format!("Failed to set device path for disconnect: {e}"))
            })?
            .build()
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to build DeviceProxy for disconnect: {e}"))
            })?;

        device
            .disconnect()
            .await
            .map_err(|e| AppError::internal(format!("Failed to disconnect device: {e}")))?;
        Ok(())
    }
}
