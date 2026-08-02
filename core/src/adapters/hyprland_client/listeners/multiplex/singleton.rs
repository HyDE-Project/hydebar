//! The one multiplexer per process and the supervisor that feeds it.
//!
//! The singleton is published only after its supervisor is running: a
//! multiplexer nobody feeds would leave every subscriber waiting forever on
//! channels that never speak. The supervisor keeps the one connection alive
//! for the lifetime of the process, reconnecting with the backoff of the
//! configuration the first subscriber brought.

use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use hydebar_proto::ports::hyprland::{
    HyprlandKeyboardEvent, HyprlandWindowEvent, HyprlandWorkspaceEvent
};
use log::warn;
use tokio::{sync::broadcast, time::sleep};

use super::wiring::build_listener;
use crate::adapters::hyprland_client::{
    HyprlandClient,
    config::HyprlandClientConfig,
    listeners::{runtime, supervisor::restart_delay}
};

/// Events a slow subscriber may fall behind before it starts losing them.
const FAN_OUT_CAPACITY: usize = 64;

/// The multiplexed listener, started once per process.
static MULTIPLEXER: OnceLock<Arc<Multiplexer>> = OnceLock::new();

/// Serializes the fallible start so racing first callers start exactly once.
static START_GATE: Mutex<()> = Mutex::new(());

/// Broadcast taps of the one compositor connection.
pub struct Multiplexer {
    pub(super) window:    broadcast::Sender<HyprlandWindowEvent>,
    pub(super) workspace: broadcast::Sender<HyprlandWorkspaceEvent>,
    pub(super) keyboard:  broadcast::Sender<HyprlandKeyboardEvent>,
    pub(super) reload:    broadcast::Sender<()>,
    /// The configuration the supervisor was started with.
    ///
    /// The first caller decides the reconnect policy for everyone; a later
    /// caller with different ideas deserves to know its own were discarded.
    config:               Arc<HyprlandClientConfig>
}

/// The running multiplexer, started on first use.
///
/// The singleton is published only after its supervisor is running: a
/// multiplexer nobody feeds would leave every subscriber waiting forever on
/// channels that never speak.
///
/// # Errors
///
/// Returns an error when the listener runtime cannot be started; the cell
/// stays empty, so the next subscriber retries the whole start.
pub(super) fn multiplexer(
    client: &HyprlandClient,
    config: &Arc<HyprlandClientConfig>
) -> Result<Arc<Multiplexer>, hydebar_proto::ports::hyprland::HyprlandError> {
    if let Some(mux) = MULTIPLEXER.get() {
        warn_on_discarded_config(mux, config);
        return Ok(Arc::clone(mux));
    }

    let _gate = START_GATE.lock().unwrap_or_else(PoisonError::into_inner);

    if let Some(mux) = MULTIPLEXER.get() {
        warn_on_discarded_config(mux, config);
        return Ok(Arc::clone(mux));
    }

    let handle = runtime::handle()?;

    let mux = Arc::new(Multiplexer {
        window:    broadcast::channel(FAN_OUT_CAPACITY).0,
        workspace: broadcast::channel(FAN_OUT_CAPACITY).0,
        keyboard:  broadcast::channel(FAN_OUT_CAPACITY).0,
        reload:    broadcast::channel(FAN_OUT_CAPACITY).0,
        config:    Arc::clone(config)
    });

    let supervised = Arc::clone(&mux);
    let client = client.clone();
    let config = Arc::clone(config);

    handle.spawn(async move {
        supervise(supervised, client, config).await;
    });

    Ok(Arc::clone(MULTIPLEXER.get_or_init(|| mux)))
}

/// Says plainly when a caller's configuration is not the one in force.
fn warn_on_discarded_config(mux: &Multiplexer, config: &Arc<HyprlandClientConfig>) {
    if !Arc::ptr_eq(&mux.config, config) && *mux.config != **config {
        warn!(
            target: "hydebar::hyprland",
            "the compositor listener keeps the configuration of its first subscriber; \
             this subscriber's differing reconnect policy is not in force"
        );
    }
}

/// Keeps the one connection alive for the lifetime of the process.
async fn supervise(
    mux: Arc<Multiplexer>,
    client: HyprlandClient,
    config: Arc<HyprlandClientConfig>
) {
    let mut attempt = 0_u32;

    loop {
        let mut listener = build_listener(&mux, &client);
        let started = tokio::time::Instant::now();

        match listener.start_listener_async().await {
            Ok(()) => warn!(
                target: "hydebar::hyprland",
                "compositor connection closed, reconnecting"
            ),
            Err(err) => warn!(
                target: "hydebar::hyprland",
                "compositor connection failed: {err}"
            )
        }

        if started.elapsed() >= config.listener_stability_window {
            attempt = 0;
        }

        attempt = attempt.saturating_add(1);
        sleep(restart_delay(config.retry_backoff, attempt)).await;
    }
}
