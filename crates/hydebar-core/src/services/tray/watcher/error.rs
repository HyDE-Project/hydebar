//! Failure classification for the tray watcher loop.

use masterror::AppError;

#[derive(Debug)]
pub enum TrayWatcherError {
    Connection(AppError),
    Initialization(AppError),
    EventStream(AppError)
}

impl std::fmt::Display for TrayWatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(err) => write!(f, "failed to connect to system bus: {err}"),
            Self::Initialization(err) => write!(f, "failed to initialise tray service: {err}"),
            Self::EventStream(err) => write!(f, "failed to listen for tray events: {err}")
        }
    }
}

impl std::error::Error for TrayWatcherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connection(err) | Self::Initialization(err) | Self::EventStream(err) => {
                err.source()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use masterror::AppError;

    use super::TrayWatcherError;

    #[test]
    fn error_variants_have_context() {
        let error = TrayWatcherError::EventStream(AppError::internal("failure"));
        let message = format!("{error}");
        assert!(message.contains("failed to listen"));
    }

    #[test]
    fn connection_errors_name_the_bus() {
        let error = TrayWatcherError::Connection(AppError::internal("boom"));
        let message = format!("{error}");
        assert!(message.contains("failed to connect"));
    }
}
