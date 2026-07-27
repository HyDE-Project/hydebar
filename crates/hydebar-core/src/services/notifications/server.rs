//! D-Bus interface implementing the freedesktop notifications spec.

use std::time::SystemTime;

use log::debug;
use zbus::interface;

use super::{Notification, NotificationStorage, Urgency};

/// D-Bus org.freedesktop.Notifications server implementation
pub struct NotificationsServer {
    storage: std::sync::Arc<std::sync::Mutex<NotificationStorage>>
}

impl NotificationsServer {
    pub fn new(storage: std::sync::Arc<std::sync::Mutex<NotificationStorage>>) -> Self {
        Self {
            storage
        }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationsServer {
    /// Get server information
    fn get_server_information(&self) -> (&str, &str, &str, &str) {
        ("hydebar", "RAprogramm", "0.6.7", "1.2")
    }

    /// Get server capabilities
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
            "Notification: {} - {} (icon: {}, timeout: {})",
            app_name, summary, app_icon, expire_timeout
        );

        // Parse urgency from hints
        let urgency = hints
            .get("urgency")
            .and_then(|v| v.downcast_ref::<u8>().ok())
            .map(Urgency::from)
            .unwrap_or(Urgency::Normal);

        let notification = Notification {
            id: 0, // Will be set by storage
            app_name: app_name.clone(),
            icon: app_icon,
            summary: summary.clone(),
            body: body.clone(),
            urgency: urgency.clone(),
            timestamp: SystemTime::now(),
            actions
        };

        let mut storage = self.storage.lock().unwrap();

        // Check if should show (DND mode)
        if !storage.should_show(&urgency) {
            debug!("Notification suppressed by DND: {}", summary);
            return 0;
        }

        // Handle replaces_id
        let id = if replaces_id > 0 {
            storage.remove(replaces_id);
            replaces_id
        } else {
            storage.add(notification)
        };

        // Play sound if enabled
        if storage.sounds_enabled() {
            Self::play_notification_sound(&urgency);
        }

        id
    }

    /// Close notification
    fn close_notification(&mut self, id: u32) {
        let mut storage = self.storage.lock().unwrap();
        storage.remove(id);
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

        // Try canberra first (standard freedesktop sound system)
        std::process::Command::new("canberra-gtk-play")
            .args(["-i", sound_name, "-d", "New notification"])
            .spawn()
            .ok();
    }
}
