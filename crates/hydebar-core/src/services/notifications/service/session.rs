//! One serving session on the org.freedesktop.Notifications bus.

use std::sync::Arc;

use iced::futures::{SinkExt, StreamExt, channel::mpsc::unbounded};
use log::{debug, error};
use zbus::{
    Connection,
    fdo::{RequestNameFlags, RequestNameReply}
};

use super::NotificationsService;
use crate::services::{
    ServiceEvent,
    notifications::{NotificationStorage, NotificationsError, NotificationsServer, takeover}
};

/// Why one attempt at serving the notification bus came to an end.
pub(super) enum SessionEnd {
    /// The bar dropped its receiver; there is nobody left to serve.
    UiClosed,
    /// The bus or the takeover refused; worth another knock later.
    Failed
}

/// Serves org.freedesktop.Notifications until the bus or the UI lets go.
///
/// Every failure path returns instead of dying silently: the caller retries
/// with a graded delay, so a bar started before the session bus — or beside
/// a daemon that only exits later — picks the duty up as soon as it can.
///
/// The well known name is requested with replacement in both directions: a
/// session usually starts a notification daemon of its own, so a polite
/// request would fail every time while the old daemon keeps painting its
/// popups; offering replacement back lets a daemon started afterwards take
/// the name over instead of failing the same way. A holder that refuses
/// replacement is stopped through its systemd unit — but only when it can
/// be proved to be a service of its own, never a unit that merely contains
/// it — and a refusal is retried later, because the holder may exit on its
/// own.
pub(super) async fn serve(
    storage: &Arc<std::sync::Mutex<NotificationStorage>>,
    output: &mut iced::futures::channel::mpsc::Sender<ServiceEvent<NotificationsService>>
) -> SessionEnd {
    let connection = match Connection::session().await {
        Ok(conn) => conn,
        Err(err) => {
            error!("Failed to connect to D-Bus: {err}");
            if output
                .send(ServiceEvent::Error(NotificationsError::DBusConnection(
                    err.to_string()
                )))
                .await
                .is_err()
            {
                return SessionEnd::UiClosed;
            }
            return SessionEnd::Failed;
        }
    };

    let (announce, mut announced) = unbounded();
    let server = NotificationsServer::new(Arc::clone(storage), announce);

    if let Err(err) = connection
        .object_server()
        .at("/org/freedesktop/Notifications", server)
        .await
    {
        error!("Failed to register D-Bus interface: {err}");
        if output
            .send(ServiceEvent::Error(NotificationsError::DBusInterface(
                err.to_string()
            )))
            .await
            .is_err()
        {
            return SessionEnd::UiClosed;
        }
        return SessionEnd::Failed;
    }

    if let Err(end) = claim_name(&connection, output).await {
        return end;
    }

    while let Some(event) = announced.next().await {
        if output.send(ServiceEvent::Update(event)).await.is_err() {
            debug!("the bar stopped listening for notifications");
            return SessionEnd::UiClosed;
        }
    }

    error!("the notification announcement stream ended, restarting the server");
    SessionEnd::Failed
}

/// Claims the notification name, deposing a holder that can be stopped.
///
/// # Errors
///
/// Returns how the attempt ended when ownership could not be taken: the
/// caller either stops serving or retries later.
async fn claim_name(
    connection: &Connection,
    output: &mut iced::futures::channel::mpsc::Sender<ServiceEvent<NotificationsService>>
) -> Result<(), SessionEnd> {
    let flags = RequestNameFlags::ReplaceExisting | RequestNameFlags::AllowReplacement;

    match connection
        .request_name_with_flags("org.freedesktop.Notifications", flags)
        .await
    {
        Ok(RequestNameReply::PrimaryOwner) => {
            debug!("the bar now serves the notification bus");
            Ok(())
        }
        Ok(RequestNameReply::InQueue) => {
            let Some(unit) = takeover::replaceable_unit(connection).await else {
                error!(
                    "a notification daemon the bar cannot safely replace holds the bus; \
                     stop it to let the bar draw its own popups"
                );
                return Err(SessionEnd::Failed);
            };

            if !takeover::stop(&unit).await {
                error!("{unit} holds the notification bus and would not stop");
                return Err(SessionEnd::Failed);
            }

            match connection
                .request_name_with_flags("org.freedesktop.Notifications", flags)
                .await
            {
                Ok(RequestNameReply::PrimaryOwner) => {
                    debug!("took the notification bus over from {unit}");
                    Ok(())
                }
                Ok(reply) => {
                    error!(
                        "the notification bus was not released yet ({reply:?}), knocking again"
                    );
                    Err(SessionEnd::Failed)
                }
                Err(err) => {
                    error!("the notification bus stayed out of reach: {err}");
                    Err(SessionEnd::Failed)
                }
            }
        }
        Ok(reply) => {
            error!(
                "another notification daemon holds the bus and refuses to be replaced \
                 ({reply:?}); stop it to let the bar draw its own popups"
            );
            if output
                .send(ServiceEvent::Error(NotificationsError::DBusConnection(
                    "the notification bus is held by another daemon".to_owned()
                )))
                .await
                .is_err()
            {
                return Err(SessionEnd::UiClosed);
            }
            Err(SessionEnd::Failed)
        }
        Err(err) => {
            error!("Failed to request D-Bus name: {err}");
            if output
                .send(ServiceEvent::Error(NotificationsError::DBusConnection(
                    err.to_string()
                )))
                .await
                .is_err()
            {
                return Err(SessionEnd::UiClosed);
            }
            Err(SessionEnd::Failed)
        }
    }
}
