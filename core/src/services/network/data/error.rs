//! Error type surfaced by the network service.

/// Errors surfaced by the [`NetworkService`].
///
/// # Examples
/// ```
/// use hydebar_core::services::network::NetworkServiceError;
///
/// let error = NetworkServiceError::new("failure");
/// assert_eq!(error.message(), "failure");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkServiceError {
    message: String
}

impl NetworkServiceError {
    /// Creates a new error with the provided message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into()
        }
    }

    /// Borrows the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<masterror::AppError> for NetworkServiceError {
    fn from(err: masterror::AppError) -> Self {
        Self::new(format!("{err:#}"))
    }
}
