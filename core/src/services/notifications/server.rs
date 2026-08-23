//! D-Bus interface implementing the freedesktop notifications spec.

use std::time::SystemTime;

use iced::futures::channel::mpsc::UnboundedSender;
use log::debug;
use zbus::interface;

use super::{Notification, NotificationEvent, NotificationStorage, Urgency};

/// D-Bus org.freedesktop.Notifications server implementation
pub struct NotificationsServer {
    storage:  std::sync::Arc<std::sync::Mutex<NotificationStorage>>,
    /// Where an accepted notification is announced.
    ///
    /// Storing one is not enough: the bar has to be told, otherwise a
    /// notification arrives, is filed away and never shows up on the screen.
    announce: UnboundedSender<NotificationEvent>
}

impl std::fmt::Debug for NotificationsServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationsServer")
            .finish_non_exhaustive()
    }
}

impl NotificationsServer {
    /// A server holding this storage and publishing through this sender.
    pub const fn new(
        storage: std::sync::Arc<std::sync::Mutex<NotificationStorage>>,
        announce: UnboundedSender<NotificationEvent>
    ) -> Self {
        Self {
            storage,
            announce
        }
    }

    /// Announces `event`, ignoring a listener that has gone away.
    fn announce(&self, event: NotificationEvent) {
        let _ = self.announce.unbounded_send(event);
    }

    /// The storage, survivable even after a panic under the lock.
    ///
    /// The server answers every notification the session sends; a poisoned
    /// mutex propagated here would crash the bar on the next one. The storage
    /// holds plain data, so reading what the panicking holder left is
    /// strictly better.
    fn storage(&self) -> std::sync::MutexGuard<'_, NotificationStorage> {
        self.storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationsServer {
    /// Get server information
    #[expect(
        clippy::unused_self,
        reason = "the zbus interface macro dispatches D-Bus calls through a method receiver"
    )]
    const fn get_server_information(&self) -> (&str, &str, &str, &str) {
        ("hydebar", "RAprogramm", "0.6.7", "1.2")
    }

    /// Get server capabilities
    #[expect(
        clippy::unused_self,
        reason = "the zbus interface macro dispatches D-Bus calls through a method receiver"
    )]
    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "body-markup".to_string(),
            "actions".to_string(),
            "icon-static".to_string(),
        ]
    }

    /// Notify - main method for sending notifications
    #[allow(clippy::too_many_arguments)]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the argument types mirror the D-Bus Notify signature the zbus macro deserializes into"
    )]
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "a mutable receiver makes zbus serialize calls to this method"
    )]
    fn notify(
        &mut self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: std::collections::HashMap<String, zbus::zvariant::Value<'_>>,
        expire_timeout: i32
    ) -> u32 {
        debug!(
            "Notification: {app_name} - {summary} (icon: {app_icon}, timeout: {expire_timeout})"
        );

        // Parse urgency from hints
        let urgency = hints
            .get("urgency")
            .and_then(|v| v.downcast_ref::<u8>().ok())
            .map_or(Urgency::Normal, Urgency::from);

        let notification = Notification {
            id: 0, // Will be set by storage
            app_name,
            icon: app_icon,
            summary: summary.clone(),
            body,
            urgency: urgency.clone(),
            timestamp: SystemTime::now(),
            actions,
            expire_timeout
        };

        let mut storage = self.storage();

        // Check if should show (DND mode)
        if !storage.should_show(&urgency) {
            debug!("Notification suppressed by DND: {summary}");
            return 0;
        }

        // Handle replaces_id
        let id = if replaces_id > 0 {
            storage.replace(replaces_id, notification.clone());
            replaces_id
        } else {
            storage.add(notification.clone())
        };

        drop(storage);

        self.announce(NotificationEvent::Received(Notification {
            id,
            ..notification
        }));

        let sounds_enabled = self.storage().sounds_enabled();

        if sounds_enabled {
            Self::play_notification_sound(&urgency);
        }

        id
    }

    /// Close notification
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "a mutable receiver makes zbus serialize calls to this method"
    )]
    fn close_notification(&mut self, id: u32) {
        self.storage().remove(id);
        self.announce(NotificationEvent::Closed(id));
    }
}

impl NotificationsServer {
    fn play_notification_sound(urgency: &Urgency) {
        // Use libcanberra or aplay to play sound
        let sound_name = match urgency {
            Urgency::Critical => "message-new-urgent",
            Urgency::Normal => "message-new-instant",
            Urgency::Low => "message"
        };

        // Try canberra first (standard freedesktop sound system); waiting on
        // a thread reaps the player instead of leaving a zombie per sound
        match std::process::Command::new("canberra-gtk-play")
            .args(["-i", sound_name, "-d", "New notification"])
            .spawn()
        {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(err) => {
                debug!("notification sound player is not available: {err}");
            }
        }
    }
}
