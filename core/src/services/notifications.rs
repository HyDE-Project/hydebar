//! Desktop notification service.

mod error;
mod model;
mod server;
mod service;
mod storage;
mod takeover;

#[cfg(test)]
mod tests;

pub use error::NotificationsError;
pub use model::{Notification, NotificationEvent, Urgency};
pub use server::NotificationsServer;
pub use service::NotificationsService;
pub use storage::NotificationStorage;

const MAX_NOTIFICATIONS: usize = 50;
