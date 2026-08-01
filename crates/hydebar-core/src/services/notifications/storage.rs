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
    /// Builds an empty storage whose id counter starts at `next_id`.
    ///
    /// Reaching the counter's wraparound organically takes four billion
    /// notifications; tests that exercise the wrap need to start next to
    /// it instead.
    #[cfg(test)]
    pub(crate) fn with_next_id(next_id: u32) -> Self {
        Self {
            next_id,
            ..Self::default()
        }
    }

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

        if let Some(existing) = self.notifications.iter_mut().find(|n| n.id == id) {
            *existing = notification;
        } else {
            if self.notifications.len() >= MAX_NOTIFICATIONS {
                self.notifications.pop_back();
            }

            self.notifications.push_front(notification);
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

    #[must_use]
    pub const fn get_all(&self) -> &VecDeque<Notification> {
        &self.notifications
    }

    #[must_use]
    pub fn unread_count(&self) -> usize {
        self.notifications.len()
    }

    pub const fn set_dnd(&mut self, enabled: bool) {
        self.do_not_disturb = enabled;
    }

    #[must_use]
    pub const fn is_dnd(&self) -> bool {
        self.do_not_disturb
    }

    pub const fn set_sounds(&mut self, enabled: bool) {
        self.sounds_enabled = enabled;
    }

    #[must_use]
    pub const fn sounds_enabled(&self) -> bool {
        self.sounds_enabled
    }

    #[must_use]
    pub const fn should_show(&self, urgency: &Urgency) -> bool {
        if self.do_not_disturb {
            // Critical notifications bypass DND
            matches!(urgency, Urgency::Critical)
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    fn a_notification(summary: &str) -> Notification {
        Notification {
            id:        0,
            app_name:  "test-app".to_string(),
            icon:      String::new(),
            summary:   summary.to_string(),
            body:      String::new(),
            urgency:   Urgency::Normal,
            timestamp: SystemTime::now(),
            actions:   Vec::new()
        }
    }

    #[test]
    fn add_assigns_strictly_increasing_ids() {
        let mut storage = NotificationStorage::default();

        let first = storage.add(a_notification("first"));
        let second = storage.add(a_notification("second"));
        let third = storage.add(a_notification("third"));

        assert!(first < second);
        assert!(second < third);
    }

    #[test]
    fn add_stamps_the_assigned_id_onto_the_stored_entry() {
        let mut storage = NotificationStorage::default();

        let mut notification = a_notification("stamped");
        notification.id = 999;
        let id = storage.add(notification);

        assert_ne!(id, 999);
        assert_eq!(storage.get_all()[0].id, id);
    }

    #[test]
    fn add_caps_the_store_and_drops_the_oldest_entry() {
        let mut storage = NotificationStorage::default();

        for i in 0..MAX_NOTIFICATIONS + 3 {
            storage.add(a_notification(&format!("n{i}")));
        }

        assert_eq!(storage.get_all().len(), MAX_NOTIFICATIONS);
        assert_eq!(storage.get_all().front().unwrap().summary, "n52");
        assert_eq!(storage.get_all().back().unwrap().summary, "n3");
    }

    #[test]
    fn a_replaced_notification_keeps_its_id() {
        let mut storage = NotificationStorage::default();

        let id = storage.add(a_notification("original"));
        let mut replacement = a_notification("updated");
        replacement.id = id + 100;
        storage.replace(id, replacement);

        assert_eq!(storage.get_all()[0].id, id);
    }

    #[test]
    fn replace_updates_in_place_without_growing_the_store() {
        let mut storage = NotificationStorage::default();

        let id = storage.add(a_notification("progress 10%"));
        storage.add(a_notification("unrelated"));
        storage.replace(id, a_notification("progress 90%"));

        assert_eq!(storage.unread_count(), 2);
        let updated = storage.get_all().iter().find(|n| n.id == id).unwrap();
        assert_eq!(updated.summary, "progress 90%");
    }

    #[test]
    fn replace_of_an_absent_id_inserts_a_new_entry() {
        let mut storage = NotificationStorage::default();

        storage.replace(42, a_notification("fresh"));

        assert_eq!(storage.unread_count(), 1);
        assert_eq!(storage.get_all()[0].id, 42);
        assert_eq!(storage.get_all()[0].summary, "fresh");
    }

    #[test]
    fn replace_insertion_still_respects_the_cap() {
        let mut storage = NotificationStorage::default();

        for i in 0..MAX_NOTIFICATIONS {
            storage.add(a_notification(&format!("n{i}")));
        }
        storage.replace(u32::MAX, a_notification("over the cap"));

        assert_eq!(storage.get_all().len(), MAX_NOTIFICATIONS);
        assert_eq!(storage.get_all().front().unwrap().id, u32::MAX);
        assert_eq!(storage.get_all().back().unwrap().summary, "n1");
    }

    #[test]
    fn remove_on_a_replaced_id_finds_the_entry() {
        let mut storage = NotificationStorage::default();

        let id = storage.add(a_notification("volume 20%"));
        storage.replace(id, a_notification("volume 80%"));
        let removed = storage.remove(id);

        assert_eq!(removed.unwrap().summary, "volume 80%");
        assert_eq!(storage.unread_count(), 0);
    }

    #[test]
    fn remove_of_an_unknown_id_returns_none_and_keeps_the_rest() {
        let mut storage = NotificationStorage::default();

        storage.add(a_notification("kept"));

        assert!(storage.remove(12345).is_none());
        assert_eq!(storage.unread_count(), 1);
    }

    #[test]
    fn next_id_wraparound_does_not_duplicate_a_live_id() {
        let mut storage = NotificationStorage::with_next_id(u32::MAX - 2);

        let mut ids = Vec::new();
        for i in 0..6 {
            ids.push(storage.add(a_notification(&format!("n{i}"))));
        }

        let mut deduplicated = ids.clone();
        deduplicated.sort_unstable();
        deduplicated.dedup();
        assert_eq!(deduplicated.len(), ids.len());

        for id in ids {
            assert!(
                storage.remove(id).is_some(),
                "id {id} must resolve to exactly one live entry"
            );
        }
        assert_eq!(storage.unread_count(), 0);
    }

    #[test]
    fn without_dnd_every_urgency_is_shown() {
        let storage = NotificationStorage::default();

        assert!(storage.should_show(&Urgency::Low));
        assert!(storage.should_show(&Urgency::Normal));
        assert!(storage.should_show(&Urgency::Critical));
    }

    #[test]
    fn dnd_lets_only_critical_through() {
        let mut storage = NotificationStorage::default();
        storage.set_dnd(true);

        assert!(!storage.should_show(&Urgency::Low));
        assert!(!storage.should_show(&Urgency::Normal));
        assert!(storage.should_show(&Urgency::Critical));
    }
}
