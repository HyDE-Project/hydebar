//! `NetworkManager` backed implementation of the network service.

use std::{collections::HashMap, ops::Deref};

use log::debug;
use masterror::{AppError, AppResult};
use tokio::process::Command;
use zbus::zvariant::{self, OwnedObjectPath, Value};

mod events;
mod proxies;
mod queries;
mod settings_dbus;

#[cfg(test)]
mod tests;

use proxies::{ConnectionSettingsProxy, NetworkManagerProxy, WirelessDeviceProxy};
pub use settings_dbus::NetworkSettingsDbus;

use super::DeviceType;
use crate::services::{
    bluetooth::BluetoothService,
    network::{AccessPoint, KnownConnection, NetworkBackend, NetworkData}
};

#[derive(Clone)]
pub struct NetworkDbus<'a>(NetworkManagerProxy<'a>);

impl NetworkBackend for NetworkDbus<'_> {
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

        let known_connections = nm
            .known_connections_internal(&wireless_access_points)
            .await?;
        debug!("Known connections: {known_connections:?}");

        Ok(NetworkData {
            wifi_present,
            active_connections,
            wifi_enabled,
            airplane_mode,
            connectivity: nm.connectivity().await?,
            wireless_access_points,
            known_connections,
            scanning_nearby_wifi: false,
            link: Default::default(),
            last_error: None
        })
    }

    async fn set_airplane_mode(&self, enable: bool) -> AppResult<()> {
        let rfkill_res = Command::new("/usr/sbin/rfkill")
            .arg(if enable { "block" } else { "unblock" })
            .arg("bluetooth")
            .output()
            .await;

        if let Err(e) = rfkill_res {
            debug!("Failed to set bluetooth rfkill: {e}");
        } else {
            debug!("Bluetooth rfkill set successfully");
        }

        let nm = NetworkDbus::new(self.0.inner().connection()).await?;
        nm.set_wireless_enabled(!enable)
            .await
            .map_err(|e| AppError::internal(format!("Failed to set wireless enabled: {e}")))?;

        Ok(())
    }

    async fn scan_nearby_wifi(&self) -> AppResult<()> {
        for device_path in self
            .wireless_access_points()
            .await?
            .iter()
            .map(|ap| ap.path.clone())
        {
            let device = WirelessDeviceProxy::builder(self.0.inner().connection())
                .path(device_path)
                .map_err(|e| {
                    AppError::internal(format!("Failed to set WirelessDeviceProxy path: {e}"))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build WirelessDeviceProxy: {e}"))
                })?;

            device
                .request_scan(HashMap::new())
                .await
                .map_err(|e| AppError::internal(format!("Failed to request WiFi scan: {e}")))?;
        }

        Ok(())
    }

    async fn set_wifi_enabled(&self, enable: bool) -> AppResult<()> {
        self.set_wireless_enabled(enable)
            .await
            .map_err(|e| AppError::internal(format!("Failed to set WiFi enabled state: {e}")))?;
        Ok(())
    }

    async fn select_access_point(
        &mut self,
        access_point: &AccessPoint,
        password: Option<String>
    ) -> AppResult<()> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;
        let connection = settings.find_connection(&access_point.ssid).await?;

        if let Some(connection) = connection.as_ref() {
            if let Some(password) = password {
                let connection = ConnectionSettingsProxy::builder(self.0.inner().connection())
                    .path(connection)
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to set ConnectionSettingsProxy path: {e}"
                        ))
                    })?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to build ConnectionSettingsProxy: {e}"
                        ))
                    })?;

                let mut s = connection.get_settings().await.map_err(|e| {
                    AppError::internal(format!("Failed to get connection settings: {e}"))
                })?;
                if let Some(wifi_settings) = s.get_mut("802-11-wireless-security") {
                    let new_password = zvariant::Value::from(password.clone())
                        .try_to_owned()
                        .map_err(|e| {
                            AppError::internal(format!("Failed to convert password value: {e}"))
                        })?;
                    wifi_settings.insert("psk".to_string(), new_password);
                }

                connection.update(s).await.map_err(|e| {
                    AppError::internal(format!("Failed to update connection settings: {e}"))
                })?;
            }

            self.activate_connection(
                connection.clone(),
                access_point.device_path.clone(),
                OwnedObjectPath::try_from("/").map_err(|e| {
                    AppError::internal(format!("Failed to create object path: {e}"))
                })?
            )
            .await
            .map_err(|e| AppError::internal(format!("Failed to activate connection: {e}")))?;
        } else {
            let name = access_point.ssid.clone();
            debug!("Create new wifi connection: {name}");

            let mut conn_settings: HashMap<&str, HashMap<&str, zvariant::Value>> =
                HashMap::from([
                    (
                        "802-11-wireless",
                        HashMap::from([("ssid", Value::Array(name.as_bytes().into()))])
                    ),
                    (
                        "connection",
                        HashMap::from([
                            ("id", Value::Str(name.into())),
                            ("type", Value::Str("802-11-wireless".into()))
                        ])
                    )
                ]);

            if let Some(pass) = password {
                conn_settings.insert(
                    "802-11-wireless-security",
                    HashMap::from([
                        ("psk", Value::Str(pass.into())),
                        ("key-mgmt", Value::Str("wpa-psk".into()))
                    ])
                );
            }

            self.add_and_activate_connection(
                conn_settings,
                &access_point.device_path,
                &access_point.path
            )
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to add and activate connection: {e}"))
            })?;
        }

        Ok(())
    }

    async fn set_vpn(
        &self,
        connection: OwnedObjectPath,
        enable: bool
    ) -> AppResult<Vec<KnownConnection>> {
        if enable {
            debug!("Activating VPN: {connection:?}");
            self.activate_connection(
                connection,
                OwnedObjectPath::try_from("/").unwrap(),
                OwnedObjectPath::try_from("/").unwrap()
            )
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to activate VPN connection: {e}"))
            })?;
        } else {
            debug!("Deactivating VPN: {connection:?}");
            self.deactivate_connection(connection).await.map_err(|e| {
                AppError::internal(format!("Failed to deactivate VPN connection: {e}"))
            })?;
        }

        let known_connections = self.known_connections().await?;
        Ok(known_connections)
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
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let nm = NetworkManagerProxy::new(conn).await.map_err(|e| {
            AppError::internal(format!("Failed to create NetworkManagerProxy: {e}"))
        })?;

        Ok(Self(nm))
    }
}
