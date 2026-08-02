//! `NetworkBackend` implementation on top of the iwd daemon.

use log::debug;
use masterror::{AppError, AppResult};
use tokio::process::Command;
use zbus::zvariant::OwnedObjectPath;

use super::{IwdDbus, adapter::AdapterProxy};
use crate::services::network::{AccessPoint, KnownConnection, NetworkBackend, NetworkData};

mod connect;
mod snapshot;

#[allow(unused_variables)]
impl NetworkBackend for IwdDbus<'_> {
    async fn access_points(&self) -> AppResult<Vec<AccessPoint>> {
        self.wireless_access_points().await
    }

    async fn initialize_data(&self) -> AppResult<NetworkData> {
        snapshot::initialize_data(self).await
    }

    /// List known (provisioned) SSIDs
    async fn known_connections(&self) -> AppResult<Vec<KnownConnection>> {
        snapshot::known_connections(self).await
    }

    async fn scan_nearby_wifi(&self) -> AppResult<()> {
        for station in self.stations().await? {
            if station
                .scanning()
                .await
                .map_err(|e| AppError::internal(format!("Failed to check scanning state: {e}")))?
            {
                debug!("Already scanning");
                continue;
            }
            station
                .scan()
                .await
                .map_err(|e| AppError::internal(format!("Failed to start scan: {e}")))?;
        }
        Ok(())
    }

    async fn set_wifi_enabled(&self, enabled: bool) -> AppResult<()> {
        AdapterProxy::new(self.inner().connection())
            .await
            .map_err(|e| AppError::internal(format!("Failed to create AdapterProxy: {e}")))?
            .set_powered(enabled)
            .await
            .map_err(|e| AppError::internal(format!("Failed to set WiFi enabled state: {e}")))?;
        Ok(())
    }

    async fn select_access_point(
        &mut self,
        ap: &AccessPoint,
        password: Option<String>
    ) -> AppResult<()> {
        connect::select_access_point(self, ap, password).await
    }

    /// iwd does not natively support VPN management; implementing it would
    /// need additional VPN management tools.
    async fn set_vpn(
        &self,
        path: OwnedObjectPath,
        enable: bool
    ) -> AppResult<Vec<KnownConnection>> {
        Err(AppError::internal(
            "VPN management not implemented for IWD backend"
        ))
    }

    async fn set_airplane_mode(&self, airplane: bool) -> AppResult<()> {
        Command::new("/usr/sbin/rfkill")
            .arg(if airplane { "block" } else { "unblock" })
            .arg("bluetooth")
            .output()
            .await?;
        self.set_wifi_enabled(!airplane).await?;
        Ok(())
    }
}
