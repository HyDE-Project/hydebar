#![allow(async_fn_in_trait)]

pub mod iwd;
pub mod network_manager;

mod common;
pub use common::*;
use masterror::AppResult;
use zbus::zvariant::OwnedObjectPath;

use super::data::{AccessPoint, KnownConnection, NetworkData};

/// Trait defining the interface for a network backend implementation.
pub trait NetworkBackend: Send + Sync {
    /// Initializes the backend and fetches the initial network data snapshot.
    async fn initialize_data(&self) -> AppResult<NetworkData>;

    /// Toggles airplane mode for the backend.
    async fn set_airplane_mode(&self, enable: bool) -> AppResult<()>;

    /// Requests a scan for nearby Wi-Fi networks.
    async fn scan_nearby_wifi(&self) -> AppResult<()>;

    /// Reads the access points the wireless devices can currently see.
    ///
    /// Every entry costs a bus round trip, and the list is drawn nowhere but
    /// inside the network menu, so the bar asks for it only while somebody is
    /// looking at that menu rather than on every signal the daemon emits.
    async fn access_points(&self) -> AppResult<Vec<AccessPoint>>;

    /// Enables or disables Wi-Fi functionality on the backend.
    async fn set_wifi_enabled(&self, enable: bool) -> AppResult<()>;

    /// Connects to a specific access point, optionally using a password.
    async fn select_access_point(
        &mut self,
        ap: &AccessPoint,
        password: Option<String>
    ) -> AppResult<()>;

    /// Retrieves the known connections from the backend.
    async fn known_connections(&self) -> AppResult<Vec<KnownConnection>>;

    /// Enables or disables a VPN connection.
    async fn set_vpn(
        &self,
        connection_path: OwnedObjectPath,
        enable: bool
    ) -> AppResult<Vec<KnownConnection>>;
}
