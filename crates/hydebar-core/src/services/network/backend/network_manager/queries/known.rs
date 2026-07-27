//! Connection profiles already stored by NetworkManager.

use std::ops::Deref;

use log::warn;
use masterror::{AppError, AppResult};
use zbus::zvariant::Value;

use super::super::{NetworkDbus, NetworkSettingsDbus, proxies::ConnectionSettingsProxy};
use crate::services::network::{AccessPoint, KnownConnection, Vpn};

impl<'a> NetworkDbus<'a> {
    pub async fn known_connections_internal(
        &self,
        wireless_access_points: &[AccessPoint]
    ) -> AppResult<Vec<KnownConnection>> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;

        let known_connections = settings.know_connections().await?;

        let mut known_ssid = Vec::with_capacity(known_connections.len());
        let mut known_vpn = Vec::new();
        for c in known_connections {
            let cs = ConnectionSettingsProxy::builder(self.0.inner().connection())
                .path(c.clone())
                .map_err(|e| {
                    AppError::internal(format!(
                        "Failed to set ConnectionSettingsProxy path: {}",
                        e
                    ))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build ConnectionSettingsProxy: {}", e))
                })?;
            let Ok(s) = cs.get_settings().await else {
                warn!("Failed to get settings for connection {c}");
                continue;
            };

            let wifi = s.get("802-11-wireless");

            if wifi.is_some() {
                let ssid =
                    s.get("connection")
                        .and_then(|c| c.get("id"))
                        .map(|s| match s.deref() {
                            Value::Str(v) => v.to_string(),
                            _ => "".to_string()
                        });

                if let Some(cur_ssid) = ssid {
                    known_ssid.push(cur_ssid);
                }
            } else if s.contains_key("vpn") {
                let id = s
                    .get("connection")
                    .and_then(|c| c.get("id"))
                    .map(|v| match v.deref() {
                        Value::Str(v) => v.to_string(),
                        _ => "".to_string()
                    });

                if let Some(id) = id {
                    known_vpn.push(Vpn {
                        name: id, path: c
                    });
                }
            }
        }
        let known_connections: Vec<_> = wireless_access_points
            .iter()
            .filter_map(|a| {
                if known_ssid.contains(&a.ssid) {
                    Some(KnownConnection::AccessPoint(a.clone()))
                } else {
                    None
                }
            })
            .chain(known_vpn.into_iter().map(KnownConnection::Vpn))
            .collect();

        Ok(known_connections)
    }
}
