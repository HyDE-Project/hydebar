//! Access to stored `NetworkManager` connection profiles.

use std::ops::Deref;

use masterror::AppResult;
use zbus::zvariant::{OwnedObjectPath, Value};

use super::proxies::{ConnectionSettingsProxy, SettingsProxy};
use crate::services::bus::bus_failure;

#[derive(Clone)]
pub struct NetworkSettingsDbus<'a>(SettingsProxy<'a>);

impl<'a> Deref for NetworkSettingsDbus<'a> {
    type Target = SettingsProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl NetworkSettingsDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let settings = SettingsProxy::new(conn)
            .await
            .map_err(|e| bus_failure("Failed to create SettingsProxy", &e))?;

        Ok(Self(settings))
    }

    pub async fn know_connections(&self) -> AppResult<Vec<OwnedObjectPath>> {
        self.list_connections()
            .await
            .map_err(|e| bus_failure("Failed to list connections", &e))
    }

    pub async fn find_connection(&self, name: &str) -> AppResult<Option<OwnedObjectPath>> {
        let connections = self
            .list_connections()
            .await
            .map_err(|e| bus_failure("Failed to list connections", &e))?;

        for connection in connections {
            let connection = ConnectionSettingsProxy::builder(self.inner().connection())
                .path(connection)
                .map_err(|e| bus_failure("Failed to set ConnectionSettingsProxy path", &e))?
                .build()
                .await
                .map_err(|e| bus_failure("Failed to build ConnectionSettingsProxy", &e))?;

            let s = connection
                .get_settings()
                .await
                .map_err(|e| bus_failure("Failed to get connection settings", &e))?;
            let Some(id) = s
                .get("connection")
                .and_then(|section| section.get("id"))
                .map(|v| match &**v {
                    Value::Str(v) => v.to_string(),
                    _ => String::new()
                })
            else {
                continue;
            };

            if id == name {
                return Ok(Some(connection.inner().path().to_owned().into()));
            }
        }

        Ok(None)
    }
}
