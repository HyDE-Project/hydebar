//! Bounded store of received notifications.

use std::collections::VecDeque;

use super::{MAX_NOTIFICATIONS, Notification, Urgency};

#[derive(Debug, Clone)]
pub struct NotificationStorage {
    notifications:  VecDeque<Notification>,
    next_id:        u32,
    do_not_disturb: bool,
    sounds_enabled: bool
}

impl Default for NotificationStorage {
    fn default() -> Self {
        Self {
            notifications:  VecDeque::with_capacity(MAX_NOTIFICATIONS),
            next_id:        1,
            do_not_disturb: false,
            sounds_enabled: true
        }
    }
}

impl NotificationStorage {
    pub fn add(&mut self, mut notification: Notification) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        notification.id = id;

        // Keep only MAX_NOTIFICATIONS
        if self.notifications.len() >= MAX_NOTIFICATIONS {
            self.notifications.pop_back();
        }

        self.notifications.push_front(notification);
        id
    }

    /// Replaces the notification with `id` in place, keeping that id.
    ///
    /// An application updating its own notification — a progress bar, a
    /// volume change — names the id it was given; the entry must stay
    /// findable under it, or a later dismissal of that id finds nothing.
    pub fn replace(&mut self, id: u32, mut notification: Notification) {
        notification.id = id;

        match self.notifications.iter_mut().find(|n| n.id == id) {
            Some(existing) => *existing = notification,
            None => {
                if self.notifications.len() >= MAX_NOTIFICATIONS {
                    self.notifications.pop_back();
                }

                self.notifications.push_front(notification);
            }
        }
    }

    pub fn remove(&mut self, id: u32) -> Option<Notification> {
        if let Some(pos) = self.notifications.iter().position(|n| n.id == id) {
            self.notifications.remove(pos)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.notifications.clear();
    }

    pub fn get_all(&self) -> &VecDeque<Notification> {
        &self.notifications
    }

    pub fn unread_count(&self) -> usize {
        self.notifications.len()
    }

    pub fn set_dnd(&mut self, enabled: bool) {
        self.do_not_disturb = enabled;
    }

    pub fn is_dnd(&self) -> bool {
        self.do_not_disturb
    }

    pub fn set_sounds(&mut self, enabled: bool) {
        self.sounds_enabled = enabled;
    }

    pub fn sounds_enabled(&self) -> bool {
        self.sounds_enabled
    }

    pub fn should_show(&self, urgency: &Urgency) -> bool {
        if self.do_not_disturb {
            // Critical notifications bypass DND
            matches!(urgency, Urgency::Critical)
        } else {
            true
        }
    }
}
