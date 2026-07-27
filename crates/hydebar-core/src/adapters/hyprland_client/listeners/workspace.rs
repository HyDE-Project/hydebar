//! Listener forwarding workspace changes.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandError, HyprlandEventStream, HyprlandWorkspaceEvent};
use hyprland::event_listener::AsyncEventListener;
use log::warn;
use tokio::{runtime::Handle, sync::mpsc, time::timeout};
use tokio_stream::wrappers::ReceiverStream;

use super::{
    super::{HyprlandClient, config::HyprlandClientConfig, util::sleep_with_backoff},
    CHANNEL_CAPACITY
};

const WORKSPACE_EVENTS_OP: &str = "workspace_events";

pub(crate) fn spawn_workspace_listener(
    config: Arc<HyprlandClientConfig>
) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
    let handle = Handle::try_current()
        .map_err(|_| HyprlandError::runtime_unavailable(WORKSPACE_EVENTS_OP))?;
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let listener_timeout = config.listener_timeout;
    let retry_backoff = config.retry_backoff;

    handle.spawn(async move {
        let tx = tx;
        loop {
            let mut listener = AsyncEventListener::new();

            listener.add_workspace_added_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::Added)).await {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
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
                        if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::Changed)).await {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
                            );
                        }
                    })
                }
            });

            listener.add_workspace_added_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::Removed)).await {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
                            );
                        }
                    })
                }
            });

            listener.add_workspace_moved_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::Moved)).await {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
                            );
                        }
                    })
                }
            });

            listener.add_layer_opened_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx
                            .send(Ok(HyprlandWorkspaceEvent::SpecialChanged))
                            .await
                        {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
                            );
                        }
                    })
                }
            });

            listener.add_monitor_removed_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx
                            .send(Ok(HyprlandWorkspaceEvent::SpecialRemoved))
                            .await
                        {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
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
                        if let Err(err) = tx
                            .send(Ok(HyprlandWorkspaceEvent::WindowClosed))
                            .await
                        {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
                            );
                        }
                    })
                }
            });

            listener.add_window_opened_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx
                            .send(Ok(HyprlandWorkspaceEvent::WindowOpened))
                            .await
                        {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
                            );
                        }
                    })
                }
            });

            listener.add_window_moved_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::WindowMoved)).await {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
                            );
                        }
                    })
                }
            });

            listener.add_active_monitor_changed_handler({
                let tx = tx.clone();
                move |_| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        if let Err(err) = tx
                            .send(Ok(HyprlandWorkspaceEvent::ActiveMonitorChanged))
                            .await
                        {
                            warn!(
                                target: "hydebar::hyprland",
                                "workspace event receiver dropped (operation={}, error={err})",
                                WORKSPACE_EVENTS_OP
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
                        "workspace listener stopped unexpectedly (operation={})",
                        WORKSPACE_EVENTS_OP
                    );
                }
                Ok(Err(err)) => {
                    let send_err = tx
                        .send(Err(HyprlandClient::backend_error(WORKSPACE_EVENTS_OP, err)))
                        .await;
                    if let Err(send_err) = send_err {
                        warn!(
                            target: "hydebar::hyprland",
                            "failed to publish workspace listener error (operation={}, error={send_err})",
                            WORKSPACE_EVENTS_OP
                        );
                        break;
                    }
                }
                Err(_) => {
                    let send_err = tx
                        .send(Err(HyprlandError::Timeout {
                            operation: WORKSPACE_EVENTS_OP,
                            timeout: listener_timeout,
                        }))
                        .await;
                    if let Err(send_err) = send_err {
                        warn!(
                            target: "hydebar::hyprland",
                            "failed to publish workspace listener timeout (operation={}, error={send_err})",
                            WORKSPACE_EVENTS_OP
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
