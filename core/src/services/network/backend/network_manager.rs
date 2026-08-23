//! `NetworkManager` backed implementation of the network service.

use std::ops::Deref;

use masterror::AppResult;
use zbus::zvariant::OwnedObjectPath;

mod connect;
mod events;
mod proxies;
mod queries;
mod radio;
mod settings_dbus;
mod snapshot;

#[cfg(test)]
mod tests;

use proxies::NetworkManagerProxy;
pub use settings_dbus::NetworkSettingsDbus;

use super::DeviceType;
use crate::services::{
    bus::bus_failure,
    network::{AccessPoint, KnownConnection, NetworkBackend, NetworkData}
};

/// The conversation with the `NetworkManager` daemon.
#[derive(Clone)]
pub struct NetworkDbus<'a>(NetworkManagerProxy<'a>);

impl std::fmt::Debug for NetworkDbus<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkDbus").finish_non_exhaustive()
    }
}

impl NetworkBackend for NetworkDbus<'_> {
    async fn access_points(&self) -> AppResult<Vec<AccessPoint>> {
        self.wireless_access_points().await
    }

    async fn initialize_data(&self) -> AppResult<NetworkData> {
        snapshot::initial_data(self).await
    }

    async fn set_airplane_mode(&self, enable: bool) -> AppResult<()> {
        radio::set_airplane_mode(self, enable).await
    }

    async fn scan_nearby_wifi(&self) -> AppResult<()> {
        radio::scan_nearby_wifi(self).await
    }

    async fn set_wifi_enabled(&self, enable: bool) -> AppResult<()> {
        self.set_wireless_enabled(enable)
            .await
            .map_err(|e| bus_failure("Failed to set WiFi enabled state", &e))
    }

    async fn select_access_point(
        &mut self,
        access_point: &AccessPoint,
        password: Option<String>
    ) -> AppResult<()> {
        connect::select_access_point(self, access_point, password).await
    }

    async fn set_vpn(
        &self,
        connection: OwnedObjectPath,
        enable: bool
    ) -> AppResult<Vec<KnownConnection>> {
        radio::set_vpn(self, connection, enable).await
    }

    async fn known_connections(&self) -> AppResult<Vec<KnownConnection>> {
        let wireless_access_points = self.wireless_access_points().await?;
        self.known_connections_internal(&wireless_access_points)
            .await
    }
}

impl<'a> Deref for NetworkDbus<'a> {
    type Target = NetworkManagerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl NetworkDbus<'_> {
    /// Connects to the `NetworkManager` service on the given bus connection.
    ///
    /// # Errors
    ///
    /// Returns an error when the `NetworkManager` proxy cannot be created.
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let nm = NetworkManagerProxy::new(conn)
            .await
            .map_err(|e| bus_failure("Failed to create NetworkManagerProxy", &e))?;

        Ok(Self(nm))
    }
}
