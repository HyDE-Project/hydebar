//! Helpers shared by the state tests.

use std::sync::OnceLock;

use flexi_logger::LoggerHandle;

/// The one logger every state test shares, started on first use.
pub(in crate::app) fn test_logger() -> LoggerHandle {
    static LOGGER: OnceLock<LoggerHandle> = OnceLock::new();
    LOGGER
        .get_or_init(|| {
            flexi_logger::Logger::try_with_env_or_str("off")
                .expect("failed to configure test logger")
                .start()
                .expect("failed to start test logger")
        })
        .clone()
}
