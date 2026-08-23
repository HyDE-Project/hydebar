//! Values describing a single desktop notification.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// How much attention a notice asked for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Urgency {
    /// Worth showing, not worth interrupting for.
    Low = 0,
    /// The ordinary case.
    Normal = 1,
    /// Worth interrupting for, and it stays until dismissed.
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

/// One notice, as it arrived.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Identifier the bus addresses the notice by.
    pub id:             u32,
    /// Which application sent it.
    pub app_name:       String,
    /// The glyph or picture it asked to be drawn with.
    pub icon:           String,
    /// Its one-line heading.
    pub summary:        String,
    /// Its text, where it carries any.
    pub body:           String,
    /// How much attention the sender asked for.
    pub urgency:        Urgency,
    /// When it arrived.
    pub timestamp:      SystemTime,
    /// The buttons it offers, in pairs of key and label.
    pub actions:        Vec<String>,
    /// Lifetime the sender asked for, milliseconds; zero means never
    /// expire, negative leaves the choice to the bar.
    pub expire_timeout: i32
}

/// What the notification server has to say.
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// New notification received
    Received(Notification),
    /// Notification closed/dismissed
    Closed(u32),
    /// Action invoked on notification
    ActionInvoked(u32, String)
}
