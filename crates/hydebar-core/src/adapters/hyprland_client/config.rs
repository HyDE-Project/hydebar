use std::time::Duration;

/// Configuration options for [`HyprlandClient`](super::HyprlandClient).
///
/// # Examples
///
/// ```no_run
/// use hydebar_core::adapters::hyprland_client::{HyprlandClient, HyprlandClientConfig};
/// use hydebar_proto::ports::hyprland::HyprlandPort;
///
/// let client = HyprlandClient::with_config(HyprlandClientConfig::default());
/// assert!(client.active_window().is_ok());
/// ```
#[derive(Clone, Debug)]
pub struct HyprlandClientConfig {
    /// Maximum duration to wait for a synchronous Hyprland request to complete.
    pub request_timeout:           Duration,
    /// How long an event listener must stay connected before it counts as
    /// healthy.
    ///
    /// A listener is silent whenever the compositor is idle, so silence is not
    /// a failure and carries no deadline. What the supervisor measures instead
    /// is how long a connection survived: one that outlived this window resets
    /// the reconnect backoff, while a connection that keeps dying immediately
    /// keeps escalating it.
    pub listener_stability_window: Duration,
    /// Total number of retry attempts for synchronous Hyprland requests.
    pub retry_attempts:            u8,
    /// Base delay between retry attempts, and before the first listener
    /// reconnect.
    pub retry_backoff:             Duration
}

impl Default for HyprlandClientConfig {
    fn default() -> Self {
        Self {
            request_timeout:           Duration::from_secs(2),
            listener_stability_window: Duration::from_mins(1),
            retry_attempts:            3,
            retry_backoff:             Duration::from_millis(250)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::HyprlandClientConfig;

    #[test]
    fn default_values_are_sensible() {
        let config = HyprlandClientConfig::default();

        assert_eq!(config.request_timeout, Duration::from_secs(2));
        assert_eq!(config.listener_stability_window, Duration::from_mins(1));
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_backoff, Duration::from_millis(250));
    }
}
