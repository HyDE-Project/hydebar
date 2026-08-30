//! Helpers shared by the state tests.

use std::sync::{Arc, OnceLock};

use flexi_logger::LoggerHandle;
use hydebar_core::{
    config::Config,
    modules::system_info::{Message, SystemInfoData}
};

use super::{App, Message as AppMessage};

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

/// Replaces the machine sample with deterministic readings.
///
/// Tests that render `CpuTemp`, `Memory`, `SystemInfo` or `Processor` blocks
/// would otherwise inherit the real hardware sample taken when the app was
/// built, which has no temperature sensor on GitHub runners and can vary
/// between developer machines. Seeding the same sample through the normal
/// `Message::SystemInfo` path makes every layout see the same panels.
pub(in crate::app) fn seed_machine_readings(app: &mut App) {
    const GIB: u64 = 1024 * 1024 * 1024;

    let data = SystemInfoData {
        cpu_usage: 12,
        cpu_count: 8,
        memory_usage: 34,
        memory_used: 8 * GIB,
        memory_total: 32 * GIB,
        memory_cached: 2 * GIB,
        memory_swap_usage: 5,
        memory_swap_used: GIB,
        memory_swap_total: 8 * GIB,
        cpu_temperature: Some(42),
        cpu_temperature_source: Some("test-sensor".to_owned()),
        cpu_model: Some("Test CPU".to_owned()),
        cpu_cores: Some(4),
        cpu_max_mhz: Some(3200),
        kernel: Some("6.0.0-test".to_owned()),
        ..SystemInfoData::default()
    };

    let _task = app.update(AppMessage::SystemInfo(Message::Sampled(Arc::new(data))));
}
