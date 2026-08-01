//! Reactive service exposing notifications to the bar.

use std::sync::Arc;

use iced::{
    Subscription,
    futures::{SinkExt, StreamExt, channel::mpsc::unbounded},
    stream
};
use log::{debug, error};
use zbus::{
    Connection,
    fdo::{RequestNameFlags, RequestNameReply}
};

use super::{
    Notification, NotificationEvent, NotificationStorage, NotificationsError, NotificationsServer,
    takeover
};
use crate::services::{ReadOnlyService, ServiceEvent};

/// Main notifications service integrating with org.freedesktop.Notifications
#[derive(Debug, Clone, Default)]
pub struct NotificationsService {
    storage: Arc<std::sync::Mutex<NotificationStorage>>
}

impl NotificationsService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            storage: Arc::new(std::sync::Mutex::new(NotificationStorage::default()))
        }
    }

    /// The storage, survivable even after a panic under the lock.
    ///
    /// A poisoned mutex here would otherwise take the notification path down
    /// for the rest of the session; the storage holds plain data, so reading
    /// whatever the panicking holder left behind is strictly better.
    fn storage(&self) -> std::sync::MutexGuard<'_, NotificationStorage> {
        self.storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[must_use]
    pub fn get_notifications(&self) -> Vec<Notification> {
        self.storage().get_all().iter().cloned().collect()
    }

    #[must_use]
    pub fn unread_count(&self) -> usize {
        self.storage().unread_count()
    }

    pub fn dismiss(&mut self, id: u32) {
        self.storage().remove(id);
    }

    pub fn clear_all(&mut self) {
        self.storage().clear();
    }

    pub fn toggle_dnd(&mut self) {
        let mut storage = self.storage();
        let current = storage.is_dnd();
        storage.set_dnd(!current);
    }

    #[must_use]
    pub fn is_dnd(&self) -> bool {
        self.storage().is_dnd()
    }
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = NotificationEvent;
    type Error = NotificationsError;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            NotificationEvent::Received(_) | NotificationEvent::ActionInvoked(_, _) => {}
            NotificationEvent::Closed(id) => {
                self.storage().remove(id);
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = std::any::TypeId::of::<Self>();
        Subscription::run_with(id, |&_id| {
            stream::channel(
                100,
                |mut output: iced::futures::channel::mpsc::Sender<ServiceEvent<Self>>| async move {
                    let storage = Arc::new(std::sync::Mutex::new(NotificationStorage::default()));
                    let service = Self {
                        storage: Arc::clone(&storage)
                    };

                    if output
                        .send(ServiceEvent::Init(service.clone()))
                        .await
                        .is_err()
                    {
                        error!("Failed to send notifications service init event");
                        return;
                    }

                    let mut failures: u32 = 0;
                    loop {
                        match serve(&storage, &mut output).await {
                            SessionEnd::UiClosed => return,
                            SessionEnd::Failed => {
                                failures = failures.saturating_add(1);
                                tokio::time::sleep(crate::services::reconnect_delay(failures))
                                    .await;
                            }
                        }
                    }
                }
            )
        })
    }
}

/// Why one attempt at serving the notification bus came to an end.
enum SessionEnd {
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
async fn serve(
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
