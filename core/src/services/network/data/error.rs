//! Error type surfaced by the network service.

use masterror::AppErrorKind;

/// Errors surfaced by the [`NetworkService`].
///
/// The kind says what sort of failure it was — a refusal, a device that is
/// gone, a daemon that never answered — so a reader of the log can tell a
/// wrong password from an absent adapter without parsing the message.
///
/// # Examples
/// ```
/// use hydebar_core::services::network::NetworkServiceError;
///
/// let error = NetworkServiceError::new("failure");
/// assert_eq!(error.message(), "failure");
/// ```
///
/// [`NetworkService`]: crate::services::network::NetworkService
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkServiceError {
    kind:    AppErrorKind,
    message: String
}

impl NetworkServiceError {
    /// Creates an error of no particular kind, carrying the message given.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind:    AppErrorKind::Internal,
            message: message.into()
        }
    }

    /// Borrows the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The sort of failure this was.
    #[must_use]
    pub const fn kind(&self) -> AppErrorKind {
        self.kind
    }
}

impl std::fmt::Display for NetworkServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.kind)
    }
}

impl std::error::Error for NetworkServiceError {}

impl From<masterror::AppError> for NetworkServiceError {
    fn from(err: masterror::AppError) -> Self {
        Self {
            kind:    err.kind,
            message: format!("{err:#}")
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use masterror::{AppError, AppErrorKind};

    use super::NetworkServiceError;

    #[test]
    fn a_refused_call_reaches_the_service_as_a_refusal() {
        let error = NetworkServiceError::from(AppError::unauthorized("no secret accepted"));

        assert_eq!(error.kind(), AppErrorKind::Unauthorized);
    }

    #[test]
    fn a_device_that_is_gone_reaches_the_service_as_an_absence() {
        let error = NetworkServiceError::from(AppError::not_found("no such device"));

        assert_eq!(error.kind(), AppErrorKind::NotFound);
    }

    #[test]
    fn the_written_form_names_the_kind_beside_the_message() {
        let error = NetworkServiceError::from(AppError::not_found("no such device"));

        assert!(error.to_string().contains("no such device"));
        assert!(error.to_string().contains("NOT_FOUND"));
    }
}
