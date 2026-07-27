//! Listener forwarding active window changes.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandError, HyprlandEventStream, HyprlandWindowEvent};
use hyprland::event_listener::AsyncEventListener;
use log::warn;
use tokio::{runtime::Handle, sync::mpsc, time::timeout};
use tokio_stream::wrappers::ReceiverStream;

use super::{
    super::{HyprlandClient, config::HyprlandClientConfig, util::sleep_with_backoff},
    CHANNEL_CAPACITY
};

const WINDOW_EVENTS_OP: &str = "window_events";

pub(crate) fn spawn_window_listener(
    config: Arc<HyprlandClientConfig>
) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
    let handle =
        Handle::try_current().map_err(|_| HyprlandError::runtime_unavailable(WINDOW_EVENTS_OP))?;
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let listener_timeout = config.listener_timeout;
    let retry_backoff = config.retry_backoff;

    handle.spawn(async move {
        let tx = tx;
        loop {
            let mut listener = AsyncEventListener::new();

            listener.add_active_window_changed_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx.send(Ok(HyprlandWindowEvent::ActiveWindowChanged)).await
                        {
                            warn!(
                                target: "hydebar::hyprland",
                                "window event receiver dropped (operation={}, error={err})",
                                WINDOW_EVENTS_OP
                            );
                        }
                    })
                }
            });

            listener.add_window_closed_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx.send(Ok(HyprlandWindowEvent::WindowClosed)).await {
                            warn!(
                                target: "hydebar::hyprland",
                                "window event receiver dropped (operation={}, error={err})",
                                WINDOW_EVENTS_OP
                            );
                        }
                    })
                }
            });

            listener.add_workspace_changed_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx
                            .send(Ok(HyprlandWindowEvent::WorkspaceFocusChanged))
                            .await
                        {
                            warn!(
                                target: "hydebar::hyprland",
                                "window event receiver dropped (operation={}, error={err})",
                                WINDOW_EVENTS_OP
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
                        "window listener stopped unexpectedly (operation={})",
                        WINDOW_EVENTS_OP
                    );
                }
                Ok(Err(err)) => {
                    let send_err = tx
                        .send(Err(HyprlandClient::backend_error(WINDOW_EVENTS_OP, err)))
                        .await;
                    if let Err(send_err) = send_err {
                        warn!(
                            target: "hydebar::hyprland",
                            "failed to publish window listener error (operation={}, error={send_err})",
                            WINDOW_EVENTS_OP
                        );
                        break;
                    }
                }
                Err(_) => {
                    let send_err = tx
                        .send(Err(HyprlandError::Timeout {
                            operation: WINDOW_EVENTS_OP,
                            timeout: listener_timeout,
                        }))
                        .await;
                    if let Err(send_err) = send_err {
                        warn!(
                            target: "hydebar::hyprland",
                            "failed to publish window listener timeout (operation={}, error={send_err})",
                            WINDOW_EVENTS_OP
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
