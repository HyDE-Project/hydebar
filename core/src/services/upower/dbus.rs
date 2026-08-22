//! Access to power devices and profiles over the `UPower` system bus.

use std::ops::Deref;

use masterror::AppResult;
use zbus::zvariant::ObjectPath;

mod battery;
mod proxies;

pub use battery::Battery;
pub use proxies::{DeviceProxy, PowerProfilesProxy, UPowerProxy};

use crate::services::bus::bus_failure;

pub struct UPowerDbus<'a>(UPowerProxy<'a>);

impl<'a> Deref for UPowerDbus<'a> {
    type Target = UPowerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UPowerDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let nm = UPowerProxy::new(conn)
            .await
            .map_err(|e| bus_failure("Failed to create UPowerProxy", &e))?;

        Ok(Self(nm))
    }

    pub async fn get_battery_devices(&self) -> AppResult<Option<Battery>> {
        let devices = self
            .enumerate_devices()
            .await
            .map_err(|e| bus_failure("Failed to enumerate UPower devices", &e))?;

        let mut res = Vec::new();

        for device in devices {
            let device = DeviceProxy::builder(self.inner().connection())
                .path(device)
                .map_err(|e| bus_failure("Failed to set DeviceProxy path", &e))?
                .build()
                .await
                .map_err(|e| bus_failure("Failed to build DeviceProxy", &e))?;

            let device_type = device
                .device_type()
                .await
                .map_err(|e| bus_failure("Failed to get device type", &e))?;
            let power_supply = device
                .power_supply()
                .await
                .map_err(|e| bus_failure("Failed to get power supply", &e))?;

            if device_type == 2 && power_supply {
                res.push(device);
            }
        }

        if res.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Battery(res)))
        }
    }

    pub async fn get_device(&self, path: &ObjectPath<'static>) -> AppResult<DeviceProxy<'static>> {
        let device = DeviceProxy::builder(self.inner().connection())
            .path(path)
            .map_err(|e| bus_failure("Failed to set DeviceProxy path", &e))?
            .build()
            .await
            .map_err(|e| bus_failure("Failed to build DeviceProxy for path", &e))?;

        Ok(device)
    }
}
