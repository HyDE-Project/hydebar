//! Claiming the watcher's bus name and tracking name ownership.

use iced::futures::StreamExt;
use log::{info, warn};
use masterror::{AppError, AppResult};
use zbus::{
    Connection,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    names::BusName,
    object_server::SignalEmitter
};

use super::server::{NAME, OBJECT_PATH, StatusNotifierWatcher};

impl StatusNotifierWatcher {
    /// Registers the watcher on the session bus and claims its well known
    /// name.
    ///
    /// Returns the connection together with the handle of the task watching
    /// bus-name ownership, so the caller owns the task's lifetime instead of
    /// leaking one detached watcher per start.
    ///
    /// # Errors
    ///
    /// Returns an error when the session bus cannot be reached, the watcher
    /// object cannot be registered, or the bus name request fails.
    pub async fn start_server() -> AppResult<(Connection, tokio::task::JoinHandle<()>)> {
        let connection = zbus::connection::Connection::session()
            .await
            .map_err(|e| AppError::internal(format!("Failed to connect to session bus: {e}")))?;
        connection
            .object_server()
            .at(OBJECT_PATH, Self::default())
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to register StatusNotifierWatcher: {e}"))
            })?;
        let interface = connection
            .object_server()
            .interface::<_, Self>(OBJECT_PATH)
            .await
            .map_err(|e| {
                AppError::internal(format!(
                    "Failed to get StatusNotifierWatcher interface: {e}"
                ))
            })?;

        let dbus_proxy = DBusProxy::new(&connection)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create DBusProxy: {e}")))?;
        let mut name_owner_changed_stream =
            dbus_proxy.receive_name_owner_changed().await.map_err(|e| {
                AppError::internal(format!("Failed to receive name owner changed signal: {e}"))
            })?;

        let flags = RequestNameFlags::AllowReplacement.into();
        if dbus_proxy
            .request_name(NAME, flags)
            .await
            .map_err(|e| AppError::internal(format!("Failed to request bus name: {e}")))?
            == RequestNameReply::InQueue
        {
            warn!("Bus name '{NAME}' already owned");
        }

        let internal_connection = connection.clone();
        let watch = tokio::spawn(async move {
            let mut have_bus_name = false;
            let unique_name = internal_connection.unique_name().map(|x| x.as_ref());
            while let Some(evt) = name_owner_changed_stream.next().await {
                let Ok(args) = evt.args() else {
                    continue;
                };
                if args.name.as_ref() == NAME {
                    if args.new_owner.as_ref() == unique_name.as_ref() {
                        info!("Acquired bus name: {NAME}");
                        have_bus_name = true;
                    } else if have_bus_name {
                        info!("Lost bus name: {NAME}");
                        have_bus_name = false;
                    }
                } else if let BusName::Unique(name) = &args.name {
                    let mut interface = interface.get_mut().await;
                    let mut services = Vec::new();

                    interface.items.retain(|(unique_name, service)| {
                        if unique_name == name {
                            services.push(service.clone());
                            false
                        } else {
                            true
                        }
                    });
                    drop(interface);

                    if services.is_empty() {
                        continue;
                    }

                    let Ok(emitter) = SignalEmitter::new(&internal_connection, OBJECT_PATH) else {
                        warn!("tray connection is gone, cannot announce the removal");
                        continue;
                    };

                    for service in services {
                        if let Err(err) =
                            Self::status_notifier_item_unregistered(&emitter, &service).await
                        {
                            warn!("failed to announce a tray item removal: {err}");
                        }
                    }
                }
            }
        });

        Ok((connection, watch))
    }
}
