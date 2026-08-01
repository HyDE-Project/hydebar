//! One compositor connection serving every event subscriber.
//!
//! The window, workspace and keyboard listeners — and the blur guard beside
//! them — used to hold a socket each, so every compositor event was parsed
//! four times by four supervisors. One multiplexed listener now owns the one
//! socket, and every subscriber taps a broadcast channel instead of the
//! compositor.

use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use hydebar_proto::ports::hyprland::{
    HyprlandEventStream, HyprlandKeyboardEvent, HyprlandPort, HyprlandWindowEvent,
    HyprlandWorkspaceEvent
};
use hyprland::{event_listener::AsyncEventListener, shared::HyprData};
use log::{error, warn};
use tokio::{sync::broadcast, time::sleep};

use super::{
    super::{HyprlandClient, config::HyprlandClientConfig},
    runtime,
    supervisor::restart_delay
};

/// Events a slow subscriber may fall behind before it starts losing them.
const FAN_OUT_CAPACITY: usize = 64;

/// The multiplexed listener, started once per process.
static MULTIPLEXER: OnceLock<Arc<Multiplexer>> = OnceLock::new();

/// Serializes the fallible start so racing first callers start exactly once.
static START_GATE: Mutex<()> = Mutex::new(());

/// Broadcast taps of the one compositor connection.
pub struct Multiplexer {
    window:    broadcast::Sender<HyprlandWindowEvent>,
    workspace: broadcast::Sender<HyprlandWorkspaceEvent>,
    keyboard:  broadcast::Sender<HyprlandKeyboardEvent>,
    reload:    broadcast::Sender<()>,
    /// The configuration the supervisor was started with.
    ///
    /// The first caller decides the reconnect policy for everyone; a later
    /// caller with different ideas deserves to know its own were discarded.
    config:    Arc<HyprlandClientConfig>
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
fn multiplexer(
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

/// The stream handed out when the multiplexer cannot start.
///
/// It ends immediately instead of hanging: every subscriber treats a closed
/// stream as a failure and retries, which retries the multiplexer too.
fn dead_stream<T: Send + 'static>(
    err: &hydebar_proto::ports::hyprland::HyprlandError
) -> HyprlandEventStream<T> {
    error!(target: "hydebar::hyprland", "compositor events unavailable: {err}");

    Box::pin(iced::futures::stream::empty())
}

/// Subscribes to the window events of the shared connection.
pub fn window_events(
    client: &HyprlandClient,
    config: &Arc<HyprlandClientConfig>
) -> HyprlandEventStream<HyprlandWindowEvent> {
    match multiplexer(client, config) {
        Ok(mux) => stream_from(mux.window.subscribe()),
        Err(err) => dead_stream(&err)
    }
}

/// Subscribes to the workspace events of the shared connection.
pub fn workspace_events(
    client: &HyprlandClient,
    config: &Arc<HyprlandClientConfig>
) -> HyprlandEventStream<HyprlandWorkspaceEvent> {
    match multiplexer(client, config) {
        Ok(mux) => stream_from(mux.workspace.subscribe()),
        Err(err) => dead_stream(&err)
    }
}

/// Subscribes to the keyboard events of the shared connection.
pub fn keyboard_events(
    client: &HyprlandClient,
    config: &Arc<HyprlandClientConfig>
) -> HyprlandEventStream<HyprlandKeyboardEvent> {
    match multiplexer(client, config) {
        Ok(mux) => stream_from(mux.keyboard.subscribe()),
        Err(err) => dead_stream(&err)
    }
}

/// Subscribes to the compositor's configuration reloads.
///
/// The receiver supports blocking reads, so the blur guard can keep its plain
/// thread while sharing the one connection with everything else.
pub fn config_reloads(
    client: &HyprlandClient,
    config: &Arc<HyprlandClientConfig>
) -> broadcast::Receiver<()> {
    match multiplexer(client, config) {
        Ok(mux) => mux.reload.subscribe(),
        Err(err) => {
            error!(target: "hydebar::hyprland", "compositor reloads unavailable: {err}");

            broadcast::channel(1).0.subscribe()
        }
    }
}

/// Wraps a broadcast tap into the stream shape the port promises.
///
/// A subscriber that fell behind is told how much it missed and keeps
/// reading: every consumer of these events re-reads the compositor state
/// anyway, so a gap costs one refresh, not correctness.
fn stream_from<T: Clone + Send + 'static>(rx: broadcast::Receiver<T>) -> HyprlandEventStream<T> {
    Box::pin(iced::futures::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => return Some((Ok(event), rx)),
                Err(broadcast::error::RecvError::Lagged(missed)) => {
                    warn!(
                        target: "hydebar::hyprland",
                        "event subscriber lagged, {missed} events were skipped"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => return None
            }
        }
    }))
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

/// Sends an event to whoever is listening; nobody listening is fine.
fn fan_out<T>(tx: &broadcast::Sender<T>, event: T) {
    let _ = tx.send(event);
}

/// Wires every handler of every domain onto one fresh connection.
#[expect(
    clippy::too_many_lines,
    reason = "declarative wiring of every Hyprland event handler; splitting would scatter the registrations"
)]
fn build_listener(mux: &Arc<Multiplexer>, client: &HyprlandClient) -> AsyncEventListener {
    let mut listener = AsyncEventListener::new();

    macro_rules! forward_window {
        ($register:ident, $event:expr) => {
            listener.$register({
                let mux = Arc::clone(mux);
                move |_| {
                    let mux = Arc::clone(&mux);
                    Box::pin(async move {
                        fan_out(&mux.window, $event);
                    })
                }
            });
        };
    }

    forward_window!(
        add_active_window_changed_handler,
        HyprlandWindowEvent::ActiveWindowChanged
    );
    forward_window!(
        add_window_title_changed_handler,
        HyprlandWindowEvent::WindowTitleChanged
    );

    macro_rules! forward_workspace {
        ($register:ident, $event:expr) => {
            listener.$register({
                let mux = Arc::clone(mux);
                move |_| {
                    let mux = Arc::clone(&mux);
                    Box::pin(async move {
                        fan_out(&mux.workspace, $event);
                    })
                }
            });
        };
    }

    forward_workspace!(add_workspace_added_handler, HyprlandWorkspaceEvent::Added);
    forward_workspace!(
        add_workspace_deleted_handler,
        HyprlandWorkspaceEvent::Removed
    );
    forward_workspace!(add_workspace_moved_handler, HyprlandWorkspaceEvent::Moved);
    forward_workspace!(
        add_changed_special_handler,
        HyprlandWorkspaceEvent::SpecialChanged
    );
    forward_workspace!(
        add_special_removed_handler,
        HyprlandWorkspaceEvent::SpecialRemoved
    );
    forward_workspace!(add_monitor_removed_handler, HyprlandWorkspaceEvent::Changed);
    forward_workspace!(
        add_window_opened_handler,
        HyprlandWorkspaceEvent::WindowOpened
    );
    forward_workspace!(
        add_window_moved_handler,
        HyprlandWorkspaceEvent::WindowMoved
    );
    forward_workspace!(
        add_active_monitor_changed_handler,
        HyprlandWorkspaceEvent::ActiveMonitorChanged
    );

    listener.add_workspace_changed_handler({
        let mux = Arc::clone(mux);
        move |_| {
            let mux = Arc::clone(&mux);
            Box::pin(async move {
                fan_out(&mux.workspace, HyprlandWorkspaceEvent::Changed);
                fan_out(&mux.window, HyprlandWindowEvent::WorkspaceFocusChanged);
            })
        }
    });

    listener.add_window_closed_handler({
        let mux = Arc::clone(mux);
        move |_| {
            let mux = Arc::clone(&mux);
            Box::pin(async move {
                fan_out(&mux.workspace, HyprlandWorkspaceEvent::WindowClosed);
                fan_out(&mux.window, HyprlandWindowEvent::WindowClosed);
            })
        }
    });

    listener.add_urgent_state_changed_handler({
        let mux = Arc::clone(mux);
        move |address| {
            let mux = Arc::clone(&mux);
            Box::pin(async move {
                let workspace_id = tokio::task::spawn_blocking(move || {
                    hyprland::data::Clients::get().ok().and_then(|clients| {
                        clients
                            .into_iter()
                            .find(|client| client.address == address)
                            .map(|client| client.workspace.id)
                    })
                })
                .await
                .ok()
                .flatten();

                if let Some(workspace_id) = workspace_id {
                    fan_out(
                        &mux.workspace,
                        HyprlandWorkspaceEvent::Urgent {
                            workspace_id
                        }
                    );
                }
            })
        }
    });

    listener.add_layout_changed_handler({
        let mux = Arc::clone(mux);
        let client = client.clone();
        move |_| {
            let mux = Arc::clone(&mux);
            let client = client.clone();
            Box::pin(async move {
                if let Ok(state) = client.keyboard_state() {
                    fan_out(
                        &mux.keyboard,
                        HyprlandKeyboardEvent::LayoutChanged(state.active_layout)
                    );
                }
            })
        }
    });

    listener.add_sub_map_changed_handler({
        let mux = Arc::clone(mux);
        move |submap| {
            let mux = Arc::clone(&mux);
            Box::pin(async move {
                let payload = if submap.trim().is_empty() {
                    None
                } else {
                    Some(submap)
                };

                fan_out(&mux.keyboard, HyprlandKeyboardEvent::SubmapChanged(payload));
            })
        }
    });

    listener.add_config_reloaded_handler({
        let mux = Arc::clone(mux);
        let client = client.clone();
        move || {
            let mux = Arc::clone(&mux);
            let client = client.clone();
            Box::pin(async move {
                fan_out(&mux.reload, ());

                if let Ok(state) = client.keyboard_state() {
                    fan_out(
                        &mux.keyboard,
                        HyprlandKeyboardEvent::LayoutConfigurationChanged(
                            state.has_multiple_layouts
                        )
                    );
                }
            })
        }
    });

    listener
}
