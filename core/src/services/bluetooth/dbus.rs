//! Adapter-level access to bluez over the system bus.

use masterror::AppResult;

use super::BluetoothState;

mod devices;
mod proxies;

pub use proxies::{AdapterProxy, BatteryProxy, BluezObjectManagerProxy};

use crate::services::bus::bus_failure;

pub struct BluetoothDbus<'a> {
    pub bluez:   BluezObjectManagerProxy<'a>,
    pub adapter: Option<AdapterProxy<'a>>
}

impl BluetoothDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let bluez = BluezObjectManagerProxy::new(conn)
            .await
            .map_err(|e| bus_failure("Failed to create BluezObjectManagerProxy", &e))?;
        let adapter = bluez
            .get_managed_objects()
            .await
            .map_err(|e| bus_failure("Failed to get managed objects", &e))?
            .into_iter()
            .find_map(|(key, item)| {
                if item.contains_key("org.bluez.Adapter1") {
                    Some(key)
                } else {
                    None
                }
            });

        let adapter = if let Some(adapter) = adapter {
            Some(
                AdapterProxy::builder(conn)
                    .path(adapter)
                    .map_err(|e| bus_failure("Failed to set adapter path", &e))?
                    .build()
                    .await
                    .map_err(|e| bus_failure("Failed to build AdapterProxy", &e))?
            )
        } else {
            None
        };

        Ok(Self {
            bluez,
            adapter
        })
    }

    pub async fn set_powered(&self, value: bool) -> AppResult<()> {
        if let Some(adapter) = &self.adapter {
            adapter
                .set_powered(value)
                .await
                .map_err(|e| bus_failure("Failed to set adapter powered state", &e))?;
        }

        Ok(())
    }

    pub async fn state(&self) -> AppResult<BluetoothState> {
        match &self.adapter {
            Some(adapter) => {
                if adapter
                    .powered()
                    .await
                    .map_err(|e| bus_failure("Failed to get adapter powered state", &e))?
                {
                    Ok(BluetoothState::Active)
                } else {
                    Ok(BluetoothState::Inactive)
                }
            }
            _ => Ok(BluetoothState::Unavailable)
        }
    }
}
