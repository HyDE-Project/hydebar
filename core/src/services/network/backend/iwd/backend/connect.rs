//! Connecting to an access point, with a password agent when one is needed.

use log::info;
use masterror::{AppError, AppResult};
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

use super::super::{IwdDbus, agents::PWAgent, network::NetworkProxy};
use crate::services::{bus::bus_failure, network::AccessPoint};

/// Connects to `ap`, provisioning the given password first when one is given.
///
/// A password is delivered through a freshly registered agent: any agent left
/// at the well-known path is unregistered, a new one is seated with a channel
/// carrying the password, and the daemon reads it from there during the
/// connection handshake.
///
/// # Errors
///
/// Returns an error when the agent cannot be registered, the password cannot
/// be handed to it, or the network refuses the connection.
pub(super) async fn select_access_point(
    iwd: &IwdDbus<'_>,
    ap: &AccessPoint,
    password: Option<String>
) -> AppResult<()> {
    let agent_manager = iwd.agent_manager().await?;

    if let Some(p) = password {
        let path: OwnedObjectPath =
            ObjectPath::from_static_str_unchecked("/hydebar/pwagent/main").into();

        match agent_manager.unregister_agent(&path).await {
            Ok(()) => info!("Successfully unregistered agent at {path}"),
            Err(e) => info!("Failed to unregister agent at {path}: {e}")
        }

        let (tx, password_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let pw_agent = PWAgent {
            password_rx
        };
        iwd.inner()
            .connection()
            .object_server()
            .at(path.clone(), pw_agent)
            .await
            .map_err(|e| bus_failure("Failed to register password agent", &e))?;

        agent_manager
            .register_agent(&path)
            .await
            .map_err(|e| bus_failure("Failed to register agent with IWD", &e))?;

        tx.send(p)
            .map_err(|e| AppError::internal(format!("Failed to send password to agent: {e}")))?;
    }

    let net = NetworkProxy::builder(iwd.inner().connection())
        .destination("net.connman.iwd")
        .map_err(|e| bus_failure("Failed to set NetworkProxy destination", &e))?
        .path(ap.path.clone())
        .map_err(|e| bus_failure("Failed to set NetworkProxy path", &e))?
        .build()
        .await
        .map_err(|e| bus_failure("Failed to build NetworkProxy", &e))?;
    net.connect()
        .await
        .map_err(|e| bus_failure("Failed to connect to network", &e))?;
    Ok(())
}
