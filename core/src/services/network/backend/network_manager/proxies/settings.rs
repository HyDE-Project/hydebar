//! Proxies for the `NetworkManager` settings interfaces.

use std::collections::HashMap;

use zbus::{OwnedObjectPath, OwnedVariant, Result, proxy};

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/AccessPoint",
    interface = "org.freedesktop.NetworkManager.AccessPoint"
)]
pub trait AccessPoint {
    #[zbus(property)]
    fn ssid(&self) -> Result<Vec<u8>>;

    #[zbus(property)]
    fn strength(&self) -> Result<u8>;

    #[zbus(property)]
    fn flags(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings",
    interface = "org.freedesktop.NetworkManager.Settings"
)]
pub trait Settings {
    fn add_connection(
        &self,
        connection: HashMap<String, HashMap<String, OwnedVariant>>
    ) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn connections(&self) -> Result<Vec<OwnedObjectPath>>;

    fn load_connections(&self, filenames: &[&str]) -> Result<(bool, Vec<String>)>;

    fn list_connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings/Connection",
    interface = "org.freedesktop.NetworkManager.Settings.Connection"
)]
pub trait ConnectionSettings {
    fn update(&self, settings: HashMap<String, HashMap<String, OwnedVariant>>) -> Result<()>;

    fn get_settings(&self) -> Result<HashMap<String, HashMap<String, OwnedVariant>>>;
}
