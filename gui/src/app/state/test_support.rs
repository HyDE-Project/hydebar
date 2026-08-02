//! Helpers shared by the state tests.

use std::sync::OnceLock;

use flexi_logger::LoggerHandle;
use hydebar_core::config::Config;

use super::App;

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

/// The bar with the stock configuration, plus whatever `shape` changes.
///
/// The runtime is leaked on purpose: the application keeps the handle it was
/// built with, and a test that draws or updates the bar outlives the scope a
/// dropped runtime would end.
pub(in crate::app) fn test_app_with(shape: impl FnOnce(&mut Config)) -> App {
    use std::{num::NonZeroUsize, path::PathBuf, sync::Arc};

    use hydebar_core::{config::ConfigManager, event_bus::EventBus, test_utils::MockHyprlandPort};
    use hydebar_proto::ports::hyprland::HyprlandPort;

    let mut config = Config::default();
    shape(&mut config);

    let port: Arc<dyn HyprlandPort> = Arc::new(MockHyprlandPort::default());
    let manager = Arc::new(ConfigManager::new(config.clone()));
    let bus = EventBus::new(NonZeroUsize::new(16).expect("a capacity of sixteen"));
    let runtime = Box::leak(Box::new(
        tokio::runtime::Runtime::new().expect("a test runtime")
    ));

    let (app, _) = App::new((
        test_logger(),
        Arc::new(config),
        manager,
        PathBuf::new(),
        port,
        bus.sender(),
        runtime.handle().clone(),
        bus.receiver()
    ));

    app
}

/// The bar with the stock configuration.
pub(in crate::app) fn test_app() -> App {
    test_app_with(|_| {})
}
