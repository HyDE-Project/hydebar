//! Listener forwarding keyboard layout and submap changes.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{
    HyprlandError, HyprlandEventStream, HyprlandKeyboardEvent, HyprlandPort
};
use hyprland::event_listener::AsyncEventListener;
use log::warn;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::{
    super::{HyprlandClient, config::HyprlandClientConfig},
    CHANNEL_CAPACITY, runtime,
    supervisor::supervise
};

const KEYBOARD_EVENTS_OP: &str = "keyboard_events";

type KeyboardSender = mpsc::Sender<Result<HyprlandKeyboardEvent, HyprlandError>>;

/// Forwards an event, reporting a consumer that walked away.
async fn publish(tx: &KeyboardSender, event: HyprlandKeyboardEvent) {
    if let Err(err) = tx.send(Ok(event)).await {
        warn!(
            target: "hydebar::hyprland",
            "keyboard event receiver dropped (operation={KEYBOARD_EVENTS_OP}, error={err})"
        );
    }
}

/// Reads the compositor state and forwards whichever event `select` derives.
async fn publish_keyboard_state<F>(tx: &KeyboardSender, client: &HyprlandClient, select: F)
where
    F: FnOnce(hydebar_proto::ports::hyprland::HyprlandKeyboardState) -> HyprlandKeyboardEvent
{
    match client.keyboard_state() {
        Ok(state) => publish(tx, select(state)).await,
        Err(err) => {
            if let Err(send_err) = tx.send(Err(err)).await {
                warn!(
                    target: "hydebar::hyprland",
                    "failed to publish keyboard state error (operation={KEYBOARD_EVENTS_OP}, error={send_err})"
                );
            }
        }
    }
}

/// Wires every handler this listener needs onto a fresh connection.
fn build_listener(client: &HyprlandClient, tx: &KeyboardSender) -> AsyncEventListener {
    let mut listener = AsyncEventListener::new();

    listener.add_layer_closed_handler({
        let tx = tx.clone();
        let client = client.clone();
        move |_| {
            let tx = tx.clone();
            let client = client.clone();
            Box::pin(async move {
                publish_keyboard_state(&tx, &client, |state| {
                    HyprlandKeyboardEvent::LayoutChanged(state.active_layout)
                })
                .await;
            })
        }
    });

    listener.add_monitor_added_handler({
        let tx = tx.clone();
        let client = client.clone();
        move |_| {
            let tx = tx.clone();
            let client = client.clone();
            Box::pin(async move {
                publish_keyboard_state(&tx, &client, |state| {
                    HyprlandKeyboardEvent::LayoutConfigurationChanged(state.has_multiple_layouts)
                })
                .await;
            })
        }
    });

    listener.add_sub_map_changed_handler({
        let tx = tx.clone();
        move |submap| {
            let tx = tx.clone();
            Box::pin(async move {
                let payload = if submap.trim().is_empty() {
                    None
                } else {
                    Some(submap)
                };

                publish(&tx, HyprlandKeyboardEvent::SubmapChanged(payload)).await;
            })
        }
    });

    listener
}

pub(crate) fn spawn_keyboard_listener(
    client: HyprlandClient,
    config: Arc<HyprlandClientConfig>
) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
    let handle = runtime::handle()?;
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let base_delay = config.retry_backoff;
    let stability_window = config.listener_stability_window;

    handle.spawn(supervise(
        KEYBOARD_EVENTS_OP,
        tx,
        base_delay,
        stability_window,
        move |tx| build_listener(&client, tx)
    ));

    Ok(Box::pin(ReceiverStream::new(rx)))
}
