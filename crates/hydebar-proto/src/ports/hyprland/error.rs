//! Error type returned by the Hyprland port operations.

use std::{error::Error, fmt, time::Duration};

/// Error type returned by [`HyprlandPort`] operations.
///
/// Each error variant stores the logical operation name to aid diagnostics.
///
/// [`HyprlandPort`]: super::HyprlandPort
#[derive(Debug)]
pub enum HyprlandError {
    /// The requested operation timed out before it could complete.
    Timeout {
        /// Logical operation identifier.
        operation: &'static str,
        /// Maximum allotted time before aborting the operation.
        timeout:   Duration
    },
    /// The backend failed to execute the requested operation.
    Backend {
        /// Logical operation identifier.
        operation: &'static str,
        /// Source error reported by the backend implementation.
        source:    Box<dyn Error + Send + Sync>
    },
    /// The async runtime required to perform the operation was unavailable.
    RuntimeUnavailable {
        /// Logical operation identifier.
        operation: &'static str
    },
    /// The requested operation is not supported by the underlying backend.
    Unsupported {
        /// Logical operation identifier.
        operation: &'static str
    },
    /// The operation failed with an explanatory message.
    Message {
        /// Logical operation identifier.
        operation: &'static str,
        /// Human readable error description.
        message:   String
    }
}

impl fmt::Display for HyprlandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout {
                operation,
                timeout
            } => {
                write!(f, "operation `{operation}` timed out after {timeout:?}")
            }
            Self::Backend {
                operation,
                source
            } => {
                write!(f, "operation `{operation}` failed: {source}")
            }
            Self::RuntimeUnavailable {
                operation
            } => {
                write!(
                    f,
                    "operation `{operation}` unavailable because no async runtime is active"
                )
            }
            Self::Unsupported {
                operation
            } => {
                write!(
                    f,
                    "operation `{operation}` not supported by this Hyprland backend"
                )
            }
            Self::Message {
                operation,
                message
            } => {
                write!(f, "operation `{operation}` failed: {message}")
            }
        }
    }
}

impl Error for HyprlandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend {
                source, ..
            } => Some(source.as_ref()),
            _ => None
        }
    }
}

impl HyprlandError {
    /// Helper for constructing [`HyprlandError::Unsupported`].
    #[must_use]
    pub const fn unsupported(operation: &'static str) -> Self {
        Self::Unsupported {
            operation
        }
    }

    /// Helper for constructing [`HyprlandError::RuntimeUnavailable`].
    #[must_use]
    pub const fn runtime_unavailable(operation: &'static str) -> Self {
        Self::RuntimeUnavailable {
            operation
        }
    }

    /// Helper for constructing [`HyprlandError::Message`].
    pub fn message(operation: &'static str, message: impl Into<String>) -> Self {
        Self::Message {
            operation,
            message: message.into()
        }
    }
}
