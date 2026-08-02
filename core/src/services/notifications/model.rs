//! Values describing a single desktop notification.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Urgency {
    Low = 0,
    Normal = 1,
    Critical = 2
}

impl From<u8> for Urgency {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Low,
            2 => Self::Critical,
            _ => Self::Normal
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id:             u32,
    pub app_name:       String,
    pub icon:           String,
    pub summary:        String,
    pub body:           String,
    pub urgency:        Urgency,
    pub timestamp:      SystemTime,
    pub actions:        Vec<String>,
    /// Lifetime the sender asked for, milliseconds; zero means never
    /// expire, negative leaves the choice to the bar.
    pub expire_timeout: i32
}

#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// New notification received
    Received(Notification),
    /// Notification closed/dismissed
    Closed(u32),
    /// Action invoked on notification
    ActionInvoked(u32, String)
}
