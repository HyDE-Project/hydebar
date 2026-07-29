//! Listener forwarding workspace changes.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandError, HyprlandEventStream, HyprlandWorkspaceEvent};
use hyprland::event_listener::AsyncEventListener;
use log::warn;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::{
    super::config::HyprlandClientConfig, CHANNEL_CAPACITY, runtime, supervisor::supervise
};

const WORKSPACE_EVENTS_OP: &str = "workspace_events";

type WorkspaceSender = mpsc::Sender<Result<HyprlandWorkspaceEvent, HyprlandError>>;

/// Forwards an event, reporting a consumer that walked away.
async fn publish(tx: &WorkspaceSender, event: HyprlandWorkspaceEvent) {
    if let Err(err) = tx.send(Ok(event)).await {
        warn!(
            target: "hydebar::hyprland",
            "workspace event receiver dropped (operation={WORKSPACE_EVENTS_OP}, error={err})"
        );
    }
}

/// Wires every handler this listener needs onto a fresh connection.
fn build_listener(tx: &WorkspaceSender) -> AsyncEventListener {
    let mut listener = AsyncEventListener::new();

    macro_rules! forward {
        ($register:ident, $event:expr) => {
            listener.$register({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        publish(&tx, $event).await;
                    })
                }
            });
        };
    }

    forward!(add_workspace_added_handler, HyprlandWorkspaceEvent::Added);
    forward!(
        add_workspace_changed_handler,
        HyprlandWorkspaceEvent::Changed
    );
    forward!(
        add_workspace_deleted_handler,
        HyprlandWorkspaceEvent::Removed
    );
    forward!(add_workspace_moved_handler, HyprlandWorkspaceEvent::Moved);
    forward!(
        add_layer_opened_handler,
        HyprlandWorkspaceEvent::SpecialChanged
    );
    forward!(
        add_monitor_removed_handler,
        HyprlandWorkspaceEvent::SpecialRemoved
    );
    forward!(
        add_window_closed_handler,
        HyprlandWorkspaceEvent::WindowClosed
    );
    forward!(
        add_window_opened_handler,
        HyprlandWorkspaceEvent::WindowOpened
    );
    forward!(
        add_window_moved_handler,
        HyprlandWorkspaceEvent::WindowMoved
    );
    forward!(
        add_active_monitor_changed_handler,
        HyprlandWorkspaceEvent::ActiveMonitorChanged
    );

    listener
}

pub(crate) fn spawn_workspace_listener(
    config: Arc<HyprlandClientConfig>
) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
    let handle = runtime::handle()?;
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let base_delay = config.retry_backoff;
    let stability_window = config.listener_stability_window;

    handle.spawn(supervise(
        WORKSPACE_EVENTS_OP,
        tx,
        base_delay,
        stability_window,
        build_listener
    ));

    Ok(Box::pin(ReceiverStream::new(rx)))
}
