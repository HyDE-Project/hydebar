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
            NotificationEvent::Received(_) => {
                // the server already stored it in the storage this service
                // shares; the event only tells the view to re-read, and
                // storing again listed every notification twice
            }
            NotificationEvent::Closed(id) => {
                self.storage().remove(id);
            }
            NotificationEvent::ActionInvoked(_, _) => {
                // Actions handling can be added later
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = std::any::TypeId::of::<Self>();
        Subscription::run_with(id, |&_id| {
            stream::channel(
                100,
                |mut output: iced::futures::channel::mpsc::Sender<ServiceEvent<Self>>| async move {
                    // Initialize storage
                    let storage = Arc::new(std::sync::Mutex::new(NotificationStorage::default()));
                    let service = Self {
                        storage: Arc::clone(&storage)
                    };

                    // Send init event
                    if output
                        .send(ServiceEvent::Init(service.clone()))
                        .await
                        .is_err()
                    {
                        error!("Failed to send notifications service init event");
                        return;
                    }

                    // Connect to session bus
                    let connection = match Connection::session().await {
                        Ok(conn) => conn,
                        Err(err) => {
                            error!("Failed to connect to D-Bus: {err}");
                            let _ = output
                                .send(ServiceEvent::Error(NotificationsError::DBusConnection(
                                    err.to_string()
                                )))
                                .await;
                            return;
                        }
                    };

                    // Create notifications server
                    let (announce, mut announced) = unbounded();
                    let server = NotificationsServer::new(Arc::clone(&storage), announce);

                    // Register D-Bus interface
                    if let Err(err) = connection
                        .object_server()
                        .at("/org/freedesktop/Notifications", server)
                        .await
                    {
                        error!("Failed to register D-Bus interface: {err}");
                        let _ = output
                            .send(ServiceEvent::Error(NotificationsError::DBusInterface(
                                err.to_string()
                            )))
                            .await;
                        return;
                    }

                    // Take the well known name, replacing whoever holds it.
                    //
                    // A session usually starts a notification daemon of its
                    // own, and it is already holding this name by the time the
                    // bar comes up. Asking politely therefore fails every time
                    // and the bar silently draws nothing while the old daemon
                    // keeps painting its own popups — which is exactly what the
                    // user did not ask for when they chose the bar's own.
                    // Replacement is also offered in return, so a daemon
                    // started afterwards can take the name back rather than
                    // fail the same way.
                    let flags =
                        RequestNameFlags::ReplaceExisting | RequestNameFlags::AllowReplacement;

                    match connection
                        .request_name_with_flags("org.freedesktop.Notifications", flags)
                        .await
                    {
                        Ok(RequestNameReply::PrimaryOwner) => {
                            debug!("the bar now serves the notification bus");
                        }
                        Ok(RequestNameReply::InQueue) => {
                            // The holder refuses to be replaced, so the request
                            // only joined a queue that never advances. The user
                            // asked for the bar's popups, so the daemon serving
                            // instead of it has to go — but only when it can be
                            // proved to be a service of its own, never a unit
                            // that merely contains it.
                            let Some(unit) = takeover::replaceable_unit(&connection).await else {
                                error!(
                                    "a notification daemon the bar cannot safely replace holds \
                                     the bus; stop it to let the bar draw its own popups"
                                );
                                return;
                            };

                            if !takeover::stop(&unit).await {
                                error!("{unit} holds the notification bus and would not stop");
                                return;
                            }

                            if let Err(err) = connection
                                .request_name_with_flags("org.freedesktop.Notifications", flags)
                                .await
                            {
                                error!("the notification bus stayed out of reach: {err}");
                                return;
                            }

                            debug!("took the notification bus over from {unit}");
                        }
                        Ok(reply) => {
                            // The name is held by a daemon that refuses to be
                            // replaced, so the request only joined a queue. The
                            // bar would then draw nothing while the old daemon
                            // keeps painting its own popups, and the user would
                            // have no idea why the setting did nothing — so say
                            // it plainly rather than wait in a queue forever.
                            error!(
                                "another notification daemon holds the bus and refuses to be \
                                 replaced ({reply:?}); stop it to let the bar draw its own popups"
                            );
                            let _ = output
                                .send(ServiceEvent::Error(NotificationsError::DBusConnection(
                                    "the notification bus is held by another daemon".to_owned()
                                )))
                                .await;
                            return;
                        }
                        Err(err) => {
                            error!("Failed to request D-Bus name: {err}");
                            let _ = output
                                .send(ServiceEvent::Error(NotificationsError::DBusConnection(
                                    err.to_string()
                                )))
                                .await;
                            return;
                        }
                    }

                    // Forward every accepted notification to the bar
                    while let Some(event) = announced.next().await {
                        if output.send(ServiceEvent::Update(event)).await.is_err() {
                            debug!("the bar stopped listening for notifications");
                            break;
                        }
                    }
                }
            )
        })
    }
}
