//! Data types describing the bluetooth adapter and its devices.

use zbus::zvariant::OwnedObjectPath;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum BluetoothState {
    Unavailable,
    Active,
    Inactive
}

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub name:      String,
    pub battery:   Option<u8>,
    pub path:      OwnedObjectPath,
    pub connected: bool
}

#[derive(Debug, Clone)]
pub struct BluetoothData {
    pub state:   BluetoothState,
    pub devices: Vec<BluetoothDevice>
}
