//! Data types describing the bluetooth adapter and its devices.

use zbus::zvariant::OwnedObjectPath;

/// What the adapter is doing.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum BluetoothState {
    /// No adapter to talk to.
    Unavailable,
    /// The adapter is on.
    Active,
    /// The adapter is there and switched off.
    Inactive
}

/// One device the adapter knows.
#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    /// Name the device announces itself under.
    pub name:      String,
    /// Charge left in the device, where it reports one.
    pub battery:   Option<u8>,
    /// Where the daemon keeps the device on the bus.
    pub path:      OwnedObjectPath,
    /// Whether the machine is talking to it now.
    pub connected: bool
}

/// What the bluetooth service currently holds.
#[derive(Debug, Clone)]
pub struct BluetoothData {
    /// What the adapter is doing.
    pub state:   BluetoothState,
    /// The devices the adapter knows.
    pub devices: Vec<BluetoothDevice>
}
