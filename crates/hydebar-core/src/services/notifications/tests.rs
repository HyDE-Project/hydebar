//! Unit tests for notification storage.

use std::time::SystemTime;

use super::{MAX_NOTIFICATIONS, Notification, NotificationStorage, Urgency};

#[test]
fn storage_max_capacity() {
    let mut storage = NotificationStorage::default();

    // Add MAX_NOTIFICATIONS + 10
    for i in 0..MAX_NOTIFICATIONS + 10 {
        let notif = Notification {
            id:        0,
            app_name:  format!("app{i}"),
            icon:      String::new(),
            summary:   format!("Summary {i}"),
            body:      String::new(),
            urgency:   Urgency::Normal,
            timestamp: SystemTime::now(),
            actions:   vec![]
        };
        storage.add(notif);
    }

    assert_eq!(storage.get_all().len(), MAX_NOTIFICATIONS);
}

#[test]
fn dnd_blocks_normal_notifications() {
    let mut storage = NotificationStorage::default();
    storage.set_dnd(true);

    assert!(!storage.should_show(&Urgency::Normal));
    assert!(!storage.should_show(&Urgency::Low));
    assert!(storage.should_show(&Urgency::Critical));
}

#[test]
fn remove_notification_by_id() {
    let mut storage = NotificationStorage::default();
    let notif = Notification {
        id:        0,
        app_name:  "test".to_string(),
        icon:      String::new(),
        summary:   "Test".to_string(),
        body:      String::new(),
        urgency:   Urgency::Normal,
        timestamp: SystemTime::now(),
        actions:   vec![]
    };

    let id = storage.add(notif);
    assert_eq!(storage.unread_count(), 1);

    storage.remove(id);
    assert_eq!(storage.unread_count(), 0);
}

#[test]
fn clear_all_notifications() {
    let mut storage = NotificationStorage::default();

    for i in 0..5 {
        let notif = Notification {
            id:        0,
            app_name:  format!("app{i}"),
            icon:      String::new(),
            summary:   format!("Summary {i}"),
            body:      String::new(),
            urgency:   Urgency::Normal,
            timestamp: SystemTime::now(),
            actions:   vec![]
        };
        storage.add(notif);
    }

    assert_eq!(storage.unread_count(), 5);
    storage.clear();
    assert_eq!(storage.unread_count(), 0);
}
