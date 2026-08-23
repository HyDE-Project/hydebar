//! Reactive service exposing notifications to the bar.

use std::sync::Arc;

use iced::{Subscription, futures::SinkExt, stream};
use log::error;
use session::{SessionEnd, serve};

use super::{Notification, NotificationEvent, NotificationStorage, NotificationsError};
use crate::services::{ReadOnlyService, ServiceEvent};

mod session;

/// Main notifications service integrating with org.freedesktop.Notifications
#[derive(Debug, Clone, Default)]
pub struct NotificationsService {
    storage: Arc<std::sync::Mutex<NotificationStorage>>
}

impl NotificationsService {
    /// A service holding no notices yet.
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

    /// Every notice being held, newest first.
    #[must_use]
    pub fn get_notifications(&self) -> Vec<Notification> {
        self.storage().get_all().iter().cloned().collect()
    }

    /// How many notices have not been looked at.
    #[must_use]
    pub fn unread_count(&self) -> usize {
        self.storage().unread_count()
    }

    /// Takes one notice down.
    pub fn dismiss(&mut self, id: u32) {
        self.storage().remove(id);
    }

    /// Takes every notice down.
    pub fn clear_all(&mut self) {
        self.storage().clear();
    }

    /// Turns do-not-disturb on, or off.
    pub fn toggle_dnd(&mut self) {
        let mut storage = self.storage();
        let current = storage.is_dnd();
        storage.set_dnd(!current);
    }

    /// Whether do-not-disturb is on.
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
