//! Commands the bar issues against the bluetooth adapter.

use masterror::AppResult;
use zbus::zvariant::OwnedObjectPath;

use super::{BluetoothService, BluetoothState, dbus::BluetoothDbus};
use crate::services::ServiceEvent;

/// What the bluetooth service can be told.
#[derive(Debug, Clone)]
pub enum BluetoothCommand {
    /// Turn the adapter on or off.
    Toggle,
    /// Connect to this device.
    ConnectDevice(OwnedObjectPath),
    /// Disconnect from this device.
    DisconnectDevice(OwnedObjectPath)
}

impl BluetoothService {
    async fn toggle_power(conn: &zbus::Connection, power: bool) -> AppResult<()> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        bluetooth.set_powered(power).await?;

        Ok(())
    }

    /// Executes `command`, refreshing the device list after a connect or
    /// disconnect so the reported data matches the adapter.
    pub async fn run_command(self, command: BluetoothCommand) -> Option<ServiceEvent<Self>> {
        match command {
            BluetoothCommand::Toggle => {
                if self.data.state == BluetoothState::Unavailable {
                    None
                } else {
                    let mut data = self.data.clone();
                    let powered = data.state == BluetoothState::Active;

                    let result = Self::toggle_power(&self.conn, !powered).await;

                    if result.is_ok() {
                        data.state = if powered {
                            BluetoothState::Inactive
                        } else {
                            BluetoothState::Active
                        };
                    }

                    Some(ServiceEvent::Update(data))
                }
            }
            BluetoothCommand::ConnectDevice(device_path) => {
                let bluetooth = BluetoothDbus::new(&self.conn).await.ok()?;
                bluetooth.connect_device(&device_path).await.ok()?;

                Self::initialize_data(&self.conn)
                    .await
                    .ok()
                    .map(ServiceEvent::Update)
            }
            BluetoothCommand::DisconnectDevice(device_path) => {
                let bluetooth = BluetoothDbus::new(&self.conn).await.ok()?;
                bluetooth.disconnect_device(&device_path).await.ok()?;

                Self::initialize_data(&self.conn)
                    .await
                    .ok()
                    .map(ServiceEvent::Update)
            }
        }
    }
}
