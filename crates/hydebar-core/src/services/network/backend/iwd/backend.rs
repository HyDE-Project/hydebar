//! `NetworkBackend` implementation on top of the iwd daemon.

use iced::futures::future::join_all;
use log::{debug, info};
use masterror::{AppError, AppResult};
use tokio::process::Command;
use zbus::zvariant::OwnedObjectPath;

use super::{IwdDbus, adapter::AdapterProxy, agents::PWAgent, network::NetworkProxy};
use crate::services::{
    bluetooth::BluetoothService,
    network::{
        AccessPoint, ConnectivityState, DeviceState, KnownConnection, NetworkBackend, NetworkData
    }
};

#[allow(unused_variables)]
impl NetworkBackend for IwdDbus<'_> {
    async fn access_points(&self) -> AppResult<Vec<AccessPoint>> {
        self.wireless_access_points().await
    }

    async fn initialize_data(&self) -> AppResult<NetworkData> {
        let nm = self;

        // airplane mode
        let bluetooth_soft_blocked = BluetoothService::check_rfkill_soft_block()
            .await
            .unwrap_or_default();

        let wifi_present = nm.wifi_device_present().await?;

        let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
        debug!("Wifi enabled: {wifi_enabled}");

        let airplane_mode = bluetooth_soft_blocked && !wifi_enabled;
        debug!("Airplane mode: {airplane_mode}");

        let active_connections = nm.active_connections_info().await?;
        debug!("Active connections: {active_connections:?}");

        let wireless_access_points = nm.wireless_access_points().await?;
        debug!("Wireless access points: {wireless_access_points:?}");

        let known_connections = nm.known_connections().await?;
        debug!("Known connections: {known_connections:?}");

        let is_scanning = join_all(self.stations().await?.iter().map(super::station::StationProxy::scanning))
            .await
            .into_iter()
            .filter_map(std::result::Result::ok)
            .any(|v| v);

        Ok(NetworkData {
            wifi_present,
            active_connections,
            wifi_enabled,
            airplane_mode,
            connectivity: nm
                .connectivity()
                .await?
                .into_iter()
                .map(ConnectivityState::from)
                .collect::<Vec<ConnectivityState>>()
                .into(),
            wireless_access_points,
            known_connections,
            scanning_nearby_wifi: is_scanning,
            link: crate::services::network::LinkDetails::default(),
            last_error: None
        })
    }

    /// List known (provisioned) SSIDs
    async fn known_connections(&self) -> AppResult<Vec<KnownConnection>> {
        let nets = self.reachable_networks().await?;
        let mut networks = Vec::new();
        for (n, s) in nets {
            if n.known_network().await.is_err() {
                continue;
            }
            let ssid = n
                .name()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get network name: {e}")))?;
            let path = n.inner().path().clone().into();
            let device_path = n
                .device()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get network device: {e}")))?
                .clone();
            networks.push(KnownConnection::AccessPoint(AccessPoint {
                ssid,
                path,
                device_path,
                strength: super::queries::strength_from_rssi(s),
                state: DeviceState::Unknown, // TODO:
                public: n.type_().await.map_err(|e| {
                    AppError::internal(format!("Failed to get network type: {e}"))
                })? == "open",
                working: false // TODO:
            }));
        }
        Ok(networks)
    }

    async fn scan_nearby_wifi(&self) -> AppResult<()> {
        for station in self.stations().await? {
            if station.scanning().await.map_err(|e| {
                AppError::internal(format!("Failed to check scanning state: {e}"))
            })? {
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
        // Get the agent manager
        let agent_manager = self.agent_manager().await?;

        // If password is provided, register a new agent to handle it
        if let Some(p) = password {
            let path = OwnedObjectPath::try_from("/hydebar/pwagent/main").unwrap();

            match agent_manager.unregister_agent(&path).await {
                Ok(()) => info!("Successfully unregistered agent at {path}"),
                Err(e) => info!("Failed to unregister agent at {path}: {e}")
            }

            // Create a new agent with the password
            let (tx, password_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            // Register the new agent
            let pw_agent = PWAgent {
                password_rx
            };
            self.inner()
                .connection()
                .object_server()
                .at(path.clone(), pw_agent)
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to register password agent: {e}"))
                })?;

            agent_manager.register_agent(&path).await.map_err(|e| {
                AppError::internal(format!("Failed to register agent with IWD: {e}"))
            })?;

            // Send the password to the agent channel
            tx.send(p).map_err(|e| {
                AppError::internal(format!("Failed to send password to agent: {e}"))
            })?;
        }

        let net = NetworkProxy::builder(self.inner().connection())
            .destination("net.connman.iwd")
            .map_err(|e| {
                AppError::internal(format!("Failed to set NetworkProxy destination: {e}"))
            })?
            .path(ap.path.clone())
            .map_err(|e| AppError::internal(format!("Failed to set NetworkProxy path: {e}")))?
            .build()
            .await
            .map_err(|e| AppError::internal(format!("Failed to build NetworkProxy: {e}")))?;
        net.connect()
            .await
            .map_err(|e| AppError::internal(format!("Failed to connect to network: {e}")))?;
        Ok(())
    }

    async fn set_vpn(
        &self,
        path: OwnedObjectPath,
        enable: bool
    ) -> AppResult<Vec<KnownConnection>> {
        // IWD doesn't natively support VPN management
        // This would need to be implemented with additional VPN management tools
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
