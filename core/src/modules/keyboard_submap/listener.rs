//! The compositor keyboard stream the submap indicator reads.

use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::{HyprlandKeyboardEvent, HyprlandPort};
use log::error;
use tokio::time::sleep;
use tokio_stream::StreamExt;

use super::Message;
use crate::ModuleEventSender;

/// How long a lost stream is left alone before it is asked for again.
const RETRY_DELAY: Duration = Duration::from_millis(500);

/// Publishes every submap change onto the bus until the task is dropped.
///
/// A stream that ends or refuses to start is retried rather than given up
/// on: the compositor outlives the bar's connection to it, and an indicator
/// that stopped listening would name a mode the keyboard left long ago.
pub(super) async fn run(hyprland: Arc<dyn HyprlandPort>, sender: ModuleEventSender<Message>) {
    loop {
        match hyprland.keyboard_events() {
            Ok(mut stream) => {
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(HyprlandKeyboardEvent::SubmapChanged(submap)) => {
                            sender.send(Message::SubmapChanged(submap.unwrap_or_default()));
                        }
                        Ok(_) => {}
                        Err(err) => error!("keyboard submap stream error: {err}")
                    }
                }
            }
            Err(err) => error!("failed to start keyboard submap stream: {err}")
        }

        sleep(RETRY_DELAY).await;
    }
}
