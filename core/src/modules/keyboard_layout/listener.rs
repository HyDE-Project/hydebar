//! The background listener: keyboard events in, layout messages out.

use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::{HyprlandKeyboardEvent, HyprlandPort};
use log::error;
use tokio::time::sleep;
use tokio_stream::StreamExt;

use super::Message;
use crate::ModuleEventSender;

const KEYBOARD_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Follows the compositor's keyboard events for as long as the module runs.
///
/// Layout changes and configuration changes are republished as messages; a
/// broken stream is reopened after a short delay rather than giving up.
pub(super) async fn run(hyprland: Arc<dyn HyprlandPort>, sender: ModuleEventSender<Message>) {
    loop {
        match hyprland.keyboard_events() {
            Ok(mut stream) => {
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(HyprlandKeyboardEvent::LayoutChanged(layout)) => {
                            sender.send(Message::ActiveLayoutChanged(layout));
                        }
                        Ok(HyprlandKeyboardEvent::LayoutConfigurationChanged(flag)) => {
                            sender.send(Message::LayoutConfigChanged(flag));
                        }
                        Ok(HyprlandKeyboardEvent::SubmapChanged(_)) => {}
                        Err(err) => {
                            error!("keyboard event stream error: {err}");
                        }
                    }
                }
            }
            Err(err) => {
                error!("failed to start keyboard event stream: {err}");
            }
        }

        sleep(KEYBOARD_EVENT_RETRY_DELAY).await;
    }
}
