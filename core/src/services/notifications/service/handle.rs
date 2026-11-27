//! The notification state the bar reads and mutates.

use std::sync::Arc;

use crate::services::notifications::{Notification, NotificationStorage};

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

    /// The storage this handle reads and mutates, shared with whoever serves
    /// the bus into it.
    pub(super) fn shared_storage(&self) -> Arc<std::sync::Mutex<NotificationStorage>> {
        Arc::clone(&self.storage)
    }

    /// The storage, survivable even after a panic under the lock.
    ///
    /// A poisoned mutex here would otherwise take the notification path down
    /// for the rest of the session; the storage holds plain data, so reading
    /// whatever the panicking holder left behind is strictly better.
    pub(super) fn storage(&self) -> std::sync::MutexGuard<'_, NotificationStorage> {
        self.storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Every notice being held, newest first.
    #[must_use]
    pub fn notifications(&self) -> Vec<Notification> {
        self.storage().all().iter().cloned().collect()
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
