//! Joining an access point, with or without a stored connection for it.

use std::collections::HashMap;

use log::debug;
use masterror::{AppError, AppResult};
use zbus::zvariant::{self, OwnedObjectPath, Value};

use super::{NetworkDbus, NetworkSettingsDbus, proxies::ConnectionSettingsProxy};
use crate::services::network::AccessPoint;

/// Joins `access_point`, reusing a stored connection when one exists.
///
/// A password given for a network already known replaces the stored one before
/// the connection is raised, which is what makes a corrected password take
/// effect instead of failing against the old one forever.
///
/// # Errors
///
/// Returns an error when the stored connections cannot be read, the password
/// cannot be written back, or the connection refuses to activate.
pub(super) async fn select_access_point(
    nm: &NetworkDbus<'_>,
    access_point: &AccessPoint,
    password: Option<String>
) -> AppResult<()> {
    let settings = NetworkSettingsDbus::new(nm.inner().connection()).await?;

    match settings.find_connection(&access_point.ssid).await? {
        Some(connection) => join_known(nm, access_point, &connection, password).await,
        None => join_new(nm, access_point, password).await
    }
}

/// Raises a connection the machine already holds settings for.
async fn join_known(
    nm: &NetworkDbus<'_>,
    access_point: &AccessPoint,
    connection: &OwnedObjectPath,
    password: Option<String>
) -> AppResult<()> {
    if let Some(password) = password {
        replace_password(nm, connection, password).await?;
    }

    let root = OwnedObjectPath::try_from("/")
        .map_err(|e| AppError::internal(format!("Failed to create object path: {e}")))?;

    nm.activate_connection(connection.clone(), access_point.device_path.clone(), root)
        .await
        .map_err(|e| AppError::internal(format!("Failed to activate connection: {e}")))?;

    Ok(())
}

/// Writes a new pre-shared key into a stored connection.
async fn replace_password(
    nm: &NetworkDbus<'_>,
    connection: &OwnedObjectPath,
    password: String
) -> AppResult<()> {
    let stored = ConnectionSettingsProxy::builder(nm.inner().connection())
        .path(connection)
        .map_err(|e| {
            AppError::internal(format!("Failed to set ConnectionSettingsProxy path: {e}"))
        })?
        .build()
        .await
        .map_err(|e| {
            AppError::internal(format!("Failed to build ConnectionSettingsProxy: {e}"))
        })?;

    let mut settings = stored
        .get_settings()
        .await
        .map_err(|e| AppError::internal(format!("Failed to get connection settings: {e}")))?;

    if let Some(security) = settings.get_mut("802-11-wireless-security") {
        let key = zvariant::Value::from(password)
            .try_to_owned()
            .map_err(|e| AppError::internal(format!("Failed to convert password value: {e}")))?;

        security.insert("psk".to_string(), key);
    }

    stored
        .update(settings)
        .await
        .map_err(|e| AppError::internal(format!("Failed to update connection settings: {e}")))
}

/// Creates a connection for an access point nothing is stored for, and raises
/// it.
async fn join_new(
    nm: &NetworkDbus<'_>,
    access_point: &AccessPoint,
    password: Option<String>
) -> AppResult<()> {
    let name = access_point.ssid.clone();
    debug!("Create new wifi connection: {name}");

    let settings = new_connection_settings(&name, password);

    nm.add_and_activate_connection(settings, &access_point.device_path, &access_point.path)
        .await
        .map_err(|e| AppError::internal(format!("Failed to add and activate connection: {e}")))?;

    Ok(())
}

/// The settings a fresh wireless connection is created from.
fn new_connection_settings(
    name: &str,
    password: Option<String>
) -> HashMap<&'static str, HashMap<&'static str, Value<'static>>> {
    let mut settings = HashMap::from([
        (
            "802-11-wireless",
            HashMap::from([("ssid", Value::Array(name.as_bytes().into()))])
        ),
        (
            "connection",
            HashMap::from([
                ("id", Value::Str(name.to_owned().into())),
                ("type", Value::Str("802-11-wireless".into()))
            ])
        )
    ]);

    if let Some(password) = password {
        settings.insert(
            "802-11-wireless-security",
            HashMap::from([
                ("psk", Value::Str(password.into())),
                ("key-mgmt", Value::Str("wpa-psk".into()))
            ])
        );
    }

    settings
}
