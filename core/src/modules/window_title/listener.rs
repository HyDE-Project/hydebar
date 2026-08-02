//! The background listener: window events in, focused-window messages out.

use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::{HyprlandPort, HyprlandWindowEvent};
use log::error;
use tokio::time::sleep;
use tokio_stream::StreamExt;

use super::Message;
use crate::ModuleEventSender;

const WINDOW_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Follows the compositor's window events for as long as the module runs.
///
/// Every event that can move the focused title triggers a fresh resolution;
/// a broken stream is reopened after a short delay rather than giving up.
pub(super) async fn run(hyprland: Arc<dyn HyprlandPort>, sender: ModuleEventSender<Message>) {
    loop {
        match hyprland.window_events() {
            Ok(mut stream) => {
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(
                            HyprlandWindowEvent::ActiveWindowChanged
                            | HyprlandWindowEvent::WindowTitleChanged
                            | HyprlandWindowEvent::WindowClosed
                            | HyprlandWindowEvent::WorkspaceFocusChanged
                        ) => {
                            publish_active_window(&hyprland, &sender).await;
                        }
                        Err(err) => {
                            error!("window event stream error: {err}");
                        }
                    }
                }
            }
            Err(err) => {
                error!("failed to start window event stream: {err}");
            }
        }

        sleep(WINDOW_EVENT_RETRY_DELAY).await;
    }
}

/// Resolves the focused window off the async workers and publishes it.
///
/// The port call blocks on the compositor socket with retries; resolving it
/// here keeps both the runtime workers and the update thread free, and the
/// message arrives carrying its data instead of an order to go fetch some.
async fn publish_active_window(
    port: &Arc<dyn HyprlandPort>,
    sender: &ModuleEventSender<Message>
) {
    let port = Arc::clone(port);

    match tokio::task::spawn_blocking(move || port.active_window()).await {
        Ok(Ok(window)) => {
            sender.send(Message::TitleChanged(window));
        }
        Ok(Err(err)) => error!("failed to retrieve active window: {err}"),
        Err(err) => error!("active window task failed: {err}")
    }
}
