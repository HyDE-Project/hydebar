//! The background listener: workspace events in, bare screens out.

use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::HyprlandPort;
use iced::futures::StreamExt;
use log::error;
use tokio::time::sleep;

use super::{Message, bareness};
use crate::ModuleEventSender;

/// How long a failed event stream rests before it is reopened.
const EVENT_RETRY_DELAY: Duration = Duration::from_secs(5);

/// How long a screen has to stay clear before the desk unfolds on it.
///
/// Closing the last window of a workspace and mapping the next one is one
/// gesture to the user and two events to the bar; unfolding on the first of
/// them would flash the whole canvas between them.
const SETTLE_DELAY: Duration = Duration::from_millis(220);

/// Follows the compositor's workspace events for as long as the desk runs.
///
/// Publishes the state of the screens up front, then one answer per event; a
/// broken stream is reopened after a short delay rather than giving up.
pub(super) async fn run(hyprland: Arc<dyn HyprlandPort>, sender: ModuleEventSender<Message>) {
    let mut published = bareness::Bareness::default();

    publish(&hyprland, &sender, &mut published).await;

    loop {
        match hyprland.workspace_events() {
            Ok(mut stream) => {
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(_) => publish(&hyprland, &sender, &mut published).await,
                        Err(err) => error!("workspace event stream error: {err}")
                    }
                }
            }
            Err(err) => error!("failed to start the desk event stream: {err}")
        }

        sleep(EVENT_RETRY_DELAY).await;
    }
}

/// Reads the screens and publishes the answer, settling before it unfolds.
///
/// A screen that just cleared is asked about once more after the settle
/// delay, so the desk answers to the gesture the user made rather than to the
/// first event of it. Folding back needs no such patience: a window that
/// mapped is on the screen already.
async fn publish(
    port: &Arc<dyn HyprlandPort>,
    sender: &ModuleEventSender<Message>,
    published: &mut bareness::Bareness
) {
    let Some(state) = read(port).await else {
        return;
    };

    let settled = if state.unfolds_further_than(published) {
        sleep(SETTLE_DELAY).await;

        match read(port).await {
            Some(second) => second,
            None => return
        }
    } else {
        state
    };

    if settled == *published {
        return;
    }

    *published = settled.clone();
    sender.send(Message::ScreensChanged(settled));
}

/// Asks the compositor which of its screens are bare, off the async workers.
///
/// The port call blocks on the compositor socket with retries, so it runs on
/// the blocking pool.
async fn read(port: &Arc<dyn HyprlandPort>) -> Option<bareness::Bareness> {
    let port = Arc::clone(port);

    match tokio::task::spawn_blocking(move || port.workspace_snapshot()).await {
        Ok(Ok(snapshot)) => Some(bareness::read(&snapshot)),
        Ok(Err(err)) => {
            error!("failed to retrieve the workspace snapshot: {err}");
            None
        }
        Err(err) => {
            error!("workspace snapshot task failed: {err}");
            None
        }
    }
}
