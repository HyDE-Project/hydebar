//! Reactive service exposing notifications to the bar.

use std::sync::Arc;

use iced::{Subscription, futures::SinkExt, stream};
use log::{debug, error};
use zbus::Connection;

use super::{
    Notification, NotificationEvent, NotificationStorage, NotificationsError, NotificationsServer
};
use crate::services::{ReadOnlyService, ServiceEvent};

/// Main notifications service integrating with org.freedesktop.Notifications
#[derive(Debug, Clone, Default)]
pub struct NotificationsService {
    storage: Arc<std::sync::Mutex<NotificationStorage>>
}

impl NotificationsService {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(std::sync::Mutex::new(NotificationStorage::default()))
        }
    }

    pub fn get_notifications(&self) -> Vec<Notification> {
        self.storage
            .lock()
            .unwrap()
            .get_all()
            .iter()
            .cloned()
            .collect()
    }

    pub fn unread_count(&self) -> usize {
        self.storage.lock().unwrap().unread_count()
    }

    pub fn dismiss(&mut self, id: u32) {
        self.storage.lock().unwrap().remove(id);
    }

    pub fn clear_all(&mut self) {
        self.storage.lock().unwrap().clear();
    }

    pub fn toggle_dnd(&mut self) {
        let mut storage = self.storage.lock().unwrap();
        let current = storage.is_dnd();
        storage.set_dnd(!current);
    }

    pub fn is_dnd(&self) -> bool {
        self.storage.lock().unwrap().is_dnd()
    }
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = NotificationEvent;
    type Error = NotificationsError;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            NotificationEvent::Received(notification) => {
                self.storage.lock().unwrap().add(notification);
            }
            NotificationEvent::Closed(id) => {
                self.storage.lock().unwrap().remove(id);
            }
            NotificationEvent::ActionInvoked(_, _) => {
                // Actions handling can be added later
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = std::any::TypeId::of::<NotificationsService>();
        Subscription::run_with(id, |&_id| {
            stream::channel(
                100,
                |mut output: iced::futures::channel::mpsc::Sender<ServiceEvent<Self>>| async move {
                    // Initialize storage
                    let storage = Arc::new(std::sync::Mutex::new(NotificationStorage::default()));
                    let service = NotificationsService {
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
                    let server = NotificationsServer::new(Arc::clone(&storage));

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

                    // Request well-known name
                    if let Err(err) = connection
                        .request_name("org.freedesktop.Notifications")
                        .await
                    {
                        error!("Failed to request D-Bus name: {err}");
                        let _ = output
                            .send(ServiceEvent::Error(NotificationsError::DBusConnection(
                                err.to_string()
                            )))
                            .await;
                        return;
                    }

                    debug!("Notifications D-Bus service registered");

                    // Keep connection alive
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            )
        })
    }
}
