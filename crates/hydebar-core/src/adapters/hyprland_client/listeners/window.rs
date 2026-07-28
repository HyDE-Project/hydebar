//! Listener forwarding active window changes.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandError, HyprlandEventStream, HyprlandWindowEvent};
use hyprland::event_listener::AsyncEventListener;
use log::warn;
use tokio::{runtime::Handle, sync::mpsc};
use tokio_stream::wrappers::ReceiverStream;

use super::{super::config::HyprlandClientConfig, CHANNEL_CAPACITY, supervisor::supervise};

const WINDOW_EVENTS_OP: &str = "window_events";

type WindowSender = mpsc::Sender<Result<HyprlandWindowEvent, HyprlandError>>;

/// Wires every handler this listener needs onto a fresh connection.
fn build_listener(tx: &WindowSender) -> AsyncEventListener {
    let mut listener = AsyncEventListener::new();

    listener.add_active_window_changed_handler({
        let tx = tx.clone();
        move |_| {
            let tx = tx.clone();
            Box::pin(async move {
                publish(&tx, HyprlandWindowEvent::ActiveWindowChanged).await;
            })
        }
    });

    listener.add_window_closed_handler({
        let tx = tx.clone();
        move |_| {
            let tx = tx.clone();
            Box::pin(async move {
                publish(&tx, HyprlandWindowEvent::WindowClosed).await;
            })
        }
    });

    listener.add_workspace_changed_handler({
        let tx = tx.clone();
        move |_| {
            let tx = tx.clone();
            Box::pin(async move {
                publish(&tx, HyprlandWindowEvent::WorkspaceFocusChanged).await;
            })
        }
    });

    listener
}

/// Forwards an event, reporting a consumer that walked away.
async fn publish(tx: &WindowSender, event: HyprlandWindowEvent) {
    if let Err(err) = tx.send(Ok(event)).await {
        warn!(
            target: "hydebar::hyprland",
            "window event receiver dropped (operation={WINDOW_EVENTS_OP}, error={err})"
        );
    }
}

pub(crate) fn spawn_window_listener(
    config: Arc<HyprlandClientConfig>
) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
    let handle =
        Handle::try_current().map_err(|_| HyprlandError::runtime_unavailable(WINDOW_EVENTS_OP))?;
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let base_delay = config.retry_backoff;
    let stability_window = config.listener_stability_window;

    handle.spawn(supervise(
        WINDOW_EVENTS_OP,
        tx,
        base_delay,
        stability_window,
        build_listener
    ));

    Ok(Box::pin(ReceiverStream::new(rx)))
}
