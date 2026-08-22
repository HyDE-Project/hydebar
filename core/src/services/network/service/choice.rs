//! Selection between the `NetworkManager` and iwd backends.

use masterror::{AppError, AppErrorKind, AppResult};
use zbus::zvariant::OwnedObjectPath;

use super::super::{
    AccessPoint, KnownConnection, NetworkData,
    backend::{NetworkBackend, iwd::IwdDbus, network_manager::NetworkDbus}
};

#[derive(Debug, Copy, Clone)]
pub(super) enum BackendChoice {
    NetworkManager,
    Iwd
}

impl BackendChoice {
    pub(super) const fn with_connection(
        self,
        conn: zbus::Connection
    ) -> BackendChoiceWithConnection {
        BackendChoiceWithConnection {
            choice: self,
            conn
        }
    }
}

pub(super) struct BackendChoiceWithConnection {
    pub(super) choice: BackendChoice,
    pub(super) conn:   zbus::Connection
}

impl NetworkBackend for BackendChoiceWithConnection {
    async fn initialize_data(&self) -> AppResult<NetworkData> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn).await?.initialize_data().await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn).await?.initialize_data().await
        }
    }

    async fn set_airplane_mode(&self, enable: bool) -> AppResult<()> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .set_airplane_mode(enable)
                    .await
            }
            BackendChoice::Iwd => {
                IwdDbus::new(&self.conn)
                    .await?
                    .set_airplane_mode(enable)
                    .await
            }
        }
    }

    async fn scan_nearby_wifi(&self) -> AppResult<()> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn).await?.scan_nearby_wifi().await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn).await?.scan_nearby_wifi().await
        }
    }

    async fn access_points(&self) -> AppResult<Vec<AccessPoint>> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn).await?.access_points().await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn).await?.access_points().await
        }
    }

    async fn set_wifi_enabled(&self, enable: bool) -> AppResult<()> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .set_wifi_enabled(enable)
                    .await
            }
            BackendChoice::Iwd => {
                IwdDbus::new(&self.conn)
                    .await?
                    .set_wifi_enabled(enable)
                    .await
            }
        }
    }

    async fn select_access_point(
        &mut self,
        ap: &AccessPoint,
        password: Option<String>
    ) -> AppResult<()> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .select_access_point(ap, password)
                    .await
            }
            BackendChoice::Iwd => {
                IwdDbus::new(&self.conn)
                    .await?
                    .select_access_point(ap, password)
                    .await
            }
        }
    }

    async fn set_vpn(
        &self,
        connection_path: OwnedObjectPath,
        enable: bool
    ) -> AppResult<Vec<KnownConnection>> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .set_vpn(connection_path, enable)
                    .await
            }
            // IWD does not handle VPNs directly
            BackendChoice::Iwd => Err(AppError::new(
                AppErrorKind::NotImplemented,
                "IWD does not support VPN management"
            ))
        }
    }

    async fn known_connections(&self) -> AppResult<Vec<KnownConnection>> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .known_connections()
                    .await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn).await?.known_connections().await
        }
    }
}
