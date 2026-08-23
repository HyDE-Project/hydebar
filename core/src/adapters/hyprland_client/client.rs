//! The client handle every Hyprland operation goes through.
//!
//! [`HyprlandClient`] is a cheap clone around the shared configuration; the
//! retry policy and the event multiplexer both read from it, so every clone
//! speaks with the same backoff and taps the same connection.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::HyprlandError;

use super::{config::HyprlandClientConfig, listeners::multiplex, sync_ops::execute_with_retry};

/// [`HyprlandPort`](hydebar_proto::ports::hyprland::HyprlandPort)
/// implementation backed by the bar's own compositor client.
#[derive(Clone, Debug)]
pub struct HyprlandClient {
    pub(super) config: Arc<HyprlandClientConfig>
}

impl Default for HyprlandClient {
    fn default() -> Self {
        Self {
            config: Arc::new(HyprlandClientConfig::default())
        }
    }
}

impl HyprlandClient {
    /// Construct a new [`HyprlandClient`] using
    /// [`HyprlandClientConfig::default`].
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a [`HyprlandClient`] with the provided configuration.
    #[must_use]
    pub fn with_config(config: HyprlandClientConfig) -> Self {
        Self {
            config: Arc::new(config)
        }
    }

    pub(super) fn execute_with_retry<R, F>(
        &self,
        operation: &'static str,
        func: F
    ) -> Result<R, HyprlandError>
    where
        R: Send + 'static,
        F: Fn() -> Result<R, HyprlandError> + Send + Sync + 'static
    {
        execute_with_retry(&self.config, operation, func)
    }

    /// A blocking tap of the compositor's configuration reloads.
    #[must_use]
    pub fn config_reloads(&self) -> tokio::sync::broadcast::Receiver<()> {
        multiplex::config_reloads(self, &self.config)
    }
}
