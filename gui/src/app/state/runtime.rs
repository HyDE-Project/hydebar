//! The machinery the bar was booted with: where its files live, who logs,
//! who answers the compositor, who reloads the configuration, and who feeds
//! the event bus.
//!
//! These are set once when the binary hands the bar its dependencies and are
//! never rearranged afterwards, which is what separates them from everything
//! else the application state holds.

use std::{path::PathBuf, sync::Arc};

use flexi_logger::LoggerHandle;
use hydebar_core::{config::ConfigManager, event_bus::EventReceiver};
use hydebar_proto::ports::hyprland::HyprlandPort;

/// What the binary opened before the bar existed, kept for as long as it is.
pub struct Runtime {
    /// The file the configuration in force was read from.
    pub(crate) config_path:    PathBuf,
    /// The live logger, so a reload can move its level without a restart.
    pub(crate) logger:         LoggerHandle,
    /// Keeps the injected compositor adapter alive for the app's lifetime;
    /// the outputs facade is meant to adopt it in place of its own client.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "held until the outputs facade adopts the injected port"
        )
    )]
    pub(crate) hyprland:       Arc<dyn HyprlandPort>,
    /// The owner of the configuration file's watcher and reloads.
    pub(crate) config_manager: Arc<ConfigManager>,
    /// The bar's end of the event bus the modules publish through.
    pub(crate) bus_receiver:   EventReceiver
}
