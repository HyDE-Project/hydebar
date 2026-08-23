//! Failures raised while serving notifications.

/// Error types for `NotificationsService`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationsError {
    /// The bus could not be reached.
    DBusConnection(String),
    /// The server interface could not be registered.
    DBusInterface(String)
}

impl std::fmt::Display for NotificationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DBusConnection(msg) => write!(f, "D-Bus connection error: {msg}"),
            Self::DBusInterface(msg) => write!(f, "D-Bus interface error: {msg}")
        }
    }
}

impl std::error::Error for NotificationsError {}
