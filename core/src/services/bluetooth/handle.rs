//! The bluetooth state the bar reads and commands.

use std::ops::Deref;

use super::data::BluetoothData;

/// The conversation with the bluetooth daemon.
#[derive(Debug, Clone)]
pub struct BluetoothService {
    pub(super) conn: zbus::Connection,
    pub(super) data: BluetoothData
}

impl Deref for BluetoothService {
    type Target = BluetoothData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
