//! Generated D-Bus proxies for the `NetworkManager` service.

mod device;
mod manager;
mod settings;

pub use device::{ActiveConnectionProxy, DeviceProxy, WiredDeviceProxy, WirelessDeviceProxy};
pub use manager::NetworkManagerProxy;
pub use settings::{AccessPointProxy, ConnectionSettingsProxy, SettingsProxy};
