//! Message folding for the notifications module: events in, snapshot out.

use log::error;

use super::{Notifications, NotificationsMessage};
use crate::services::{ReadOnlyService, ServiceEvent};

impl Notifications {
    /// Update the module state based on notification events.
    pub fn update(&mut self, message: NotificationsMessage) {
        match message {
            NotificationsMessage::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                }
                ServiceEvent::Update(data) => {
                    if let Some(notifications) = self.service.as_mut() {
                        notifications.update(data);
                    }
                }
                ServiceEvent::Error(error) => {
                    error!("Notifications service error: {error}");
                }
            },
            NotificationsMessage::Dismiss(id) => {
                if let Some(service) = self.service.as_mut() {
                    service.dismiss(id);
                }
            }
            NotificationsMessage::ClearAll => {
                if let Some(service) = self.service.as_mut() {
                    service.clear_all();
                }
            }
            NotificationsMessage::ToggleDND => {
                if let Some(service) = self.service.as_mut() {
                    service.toggle_dnd();
                }
            }
        }

        self.refresh_snapshot();
    }

    /// Re-reads the store once, after something actually changed.
    fn refresh_snapshot(&mut self) {
        let Some(service) = self.service.as_ref() else {
            return;
        };

        self.list = service.get_notifications();
        self.unread = service.unread_count();
        self.dnd = service.is_dnd();
    }
}
