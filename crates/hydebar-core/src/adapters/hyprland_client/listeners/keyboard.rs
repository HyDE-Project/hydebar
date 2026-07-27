//! Listener forwarding keyboard layout and submap changes.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{
    HyprlandError, HyprlandEventStream, HyprlandKeyboardEvent, HyprlandPort
};
use hyprland::event_listener::AsyncEventListener;
use log::warn;
use tokio::{runtime::Handle, sync::mpsc, time::timeout};
use tokio_stream::wrappers::ReceiverStream;

use super::{
    super::{HyprlandClient, config::HyprlandClientConfig, util::sleep_with_backoff},
    CHANNEL_CAPACITY
};

const KEYBOARD_EVENTS_OP: &str = "keyboard_events";

pub(crate) fn spawn_keyboard_listener(
    client: HyprlandClient,
    config: Arc<HyprlandClientConfig>
) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
    let handle = Handle::try_current()
        .map_err(|_| HyprlandError::runtime_unavailable(KEYBOARD_EVENTS_OP))?;
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let listener_timeout = config.listener_timeout;
    let retry_backoff = config.retry_backoff;

    handle.spawn(async move {
        let tx = tx;
        loop {
            let mut listener = AsyncEventListener::new();

            listener.add_layer_closed_handler({
                let tx = tx.clone();
                let client = client.clone();
                move |_| {
                    let tx = tx.clone();
                    let client = client.clone();
                    Box::pin(async move {
                        match client.keyboard_state() {
                            Ok(state) => {
                                if let Err(err) = tx
                                    .send(Ok(HyprlandKeyboardEvent::LayoutChanged(state.active_layout)))
                                    .await
                                {
                                    warn!(
                                        target: "hydebar::hyprland",
                                        "keyboard event receiver dropped (operation={}, error={err})",
                                        KEYBOARD_EVENTS_OP
                                    );
                                }
                            }
                            Err(err) => {
                                if let Err(send_err) = tx.send(Err(err)).await {
                                    warn!(
                                        target: "hydebar::hyprland",
                                        "failed to publish keyboard state error (operation={}, error={send_err})",
                                        KEYBOARD_EVENTS_OP
                                    );
                                }
                            }
                        }
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
                        match client.keyboard_state() {
                            Ok(state) => {
                                if let Err(err) = tx
                                    .send(Ok(HyprlandKeyboardEvent::LayoutConfigurationChanged(
                                        state.has_multiple_layouts,
                                    )))
                                    .await
                                {
                                    warn!(
                                        target: "hydebar::hyprland",
                                        "keyboard event receiver dropped (operation={}, error={err})",
                                        KEYBOARD_EVENTS_OP
                                    );
                                }
                            }
                            Err(err) => {
                                if let Err(send_err) = tx.send(Err(err)).await {
                                    warn!(
                                        target: "hydebar::hyprland",
                                        "failed to publish keyboard config error (operation={}, error={send_err})",
                                        KEYBOARD_EVENTS_OP
                                    );
                                }
                            }
                        }
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
                        if let Err(err) = tx
                            .send(Ok(HyprlandKeyboardEvent::SubmapChanged(payload)))
                            .await
                        {
                            warn!(
                                target: "hydebar::hyprland",
                                "keyboard event receiver dropped (operation={}, error={err})",
                                KEYBOARD_EVENTS_OP
                            );
                        }
                    })
                }
            });

            let result = timeout(listener_timeout, listener.start_listener_async()).await;
            match result {
                Ok(Ok(())) => {
                    warn!(
                        target: "hydebar::hyprland",
                        "keyboard listener stopped unexpectedly (operation={})",
                        KEYBOARD_EVENTS_OP
                    );
                }
                Ok(Err(err)) => {
                    let send_err = tx
                        .send(Err(HyprlandClient::backend_error(KEYBOARD_EVENTS_OP, err)))
                        .await;
                    if let Err(send_err) = send_err {
                        warn!(
                            target: "hydebar::hyprland",
                            "failed to publish keyboard listener error (operation={}, error={send_err})",
                            KEYBOARD_EVENTS_OP
                        );
                        break;
                    }
                }
                Err(_) => {
                    let send_err = tx
                        .send(Err(HyprlandError::Timeout {
                            operation: KEYBOARD_EVENTS_OP,
                            timeout: listener_timeout,
                        }))
                        .await;
                    if let Err(send_err) = send_err {
                        warn!(
                            target: "hydebar::hyprland",
                            "failed to publish keyboard listener timeout (operation={}, error={send_err})",
                            KEYBOARD_EVENTS_OP
                        );
                        break;
                    }
                }
            }

            if tx.is_closed() {
                break;
            }

            sleep_with_backoff(retry_backoff).await;
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}
