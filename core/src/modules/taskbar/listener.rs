//! The background listener: window events in, client lists out.

use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::HyprlandPort;
use iced::futures::StreamExt;
use log::error;
use tokio::time::sleep;

use super::Message;
use crate::ModuleEventSender;

/// How long a failed event stream rests before it is reopened.
const EVENT_RETRY_DELAY: Duration = Duration::from_secs(5);

/// Follows the compositor's window events for as long as the module runs.
///
/// Publishes one client list up front, then one per window event; a broken
/// stream is reopened after a short delay rather than giving up.
pub(super) async fn run(hyprland: Arc<dyn HyprlandPort>, sender: ModuleEventSender<Message>) {
    publish_clients(&hyprland, &sender).await;

    loop {
        match hyprland.window_events() {
            Ok(mut stream) => {
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(_) => publish_clients(&hyprland, &sender).await,
                        Err(err) => error!("window event stream error: {err}")
                    }
                }
            }
            Err(err) => {
                error!("failed to start the taskbar event stream: {err}");
            }
        }

        sleep(EVENT_RETRY_DELAY).await;
    }
}

/// Resolves a fresh client list off the async workers and publishes it.
///
/// The port call blocks on the compositor socket with retries, so it runs on
/// the blocking pool. An answer equal to the previous one is still published;
/// the bus replaces the stale snapshot in place, so the cost is one message,
/// not one repaint per event.
async fn publish_clients(port: &Arc<dyn HyprlandPort>, sender: &ModuleEventSender<Message>) {
    let port = Arc::clone(port);

    match tokio::task::spawn_blocking(move || port.clients_snapshot()).await {
        Ok(Ok(clients)) => {
            sender.send(Message::ClientsChanged(clients));
        }
        Ok(Err(err)) => error!("failed to retrieve the client list: {err}"),
        Err(err) => error!("client list task failed: {err}")
    }
}
