//! The background listener: workspace events in, bare screens out.

use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::HyprlandPort;
use iced::futures::{StreamExt, future::Either, stream::Stream};
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
/// them would flash the whole canvas between them. Kept to the width of one
/// such pair of events and no wider: every millisecond of it is a wait
/// between clearing the screen and the bar moving, and a wait before a
/// movement is read as the bar being slow to notice.
const SETTLE_DELAY: Duration = Duration::from_millis(40);

/// Follows the compositor's workspace events for as long as the desk runs.
///
/// Publishes the state of the screens up front, then one answer per event; a
/// broken stream is reopened after a short delay rather than giving up.
pub(super) async fn run(hyprland: Arc<dyn HyprlandPort>, sender: ModuleEventSender<Message>) {
    let mut published = bareness::Bareness::default();

    publish(&hyprland, &sender, &mut published).await;

    loop {
        match stirrings(hyprland.as_ref()) {
            Some(mut stirrings) => {
                while stirrings.next().await.is_some() {
                    publish(&hyprland, &sender, &mut published).await;
                }
            }
            None => error!("failed to start the desk event streams")
        }

        sleep(EVENT_RETRY_DELAY).await;
    }
}

/// Every compositor event that can change which screens are bare.
///
/// Two streams, merged: the workspace one carries windows opening, closing
/// and moving between workspaces, and the window one carries the rest —
/// a window taken out of the tiling and left floating above it among them.
/// Following only the first left the desk standing while a window it should
/// have folded for was dropped into the layout.
///
/// Every event is flattened to a nudge: the answer is read from a fresh
/// snapshot either way, so what kind of stirring it was does not matter.
fn stirrings(hyprland: &dyn HyprlandPort) -> Option<impl Stream<Item = ()> + Send + use<>> {
    let workspaces = hyprland
        .workspace_events()
        .inspect_err(|err| error!("failed to follow the workspaces: {err}"))
        .ok();
    let windows = hyprland
        .window_events()
        .inspect_err(|err| error!("failed to follow the windows: {err}"))
        .ok();

    match (workspaces, windows) {
        (Some(workspaces), Some(windows)) => Some(Either::Left(tokio_stream::StreamExt::merge(
            workspaces.map(|_| ()),
            windows.map(|_| ())
        ))),
        (Some(only), None) => Some(Either::Right(Either::Left(only.map(|_| ())))),
        (None, Some(only)) => Some(Either::Right(Either::Right(only.map(|_| ())))),
        (None, None) => None
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
/// Two questions in one trip: which workspace each screen shows, and which
/// windows are mapped where. The second is what tells a window that tiled
/// into the workspace from one that merely floats over it, and only the
/// first kind takes a screen from the desk.
///
/// The port calls block on the compositor socket with retries, so they run on
/// the blocking pool.
async fn read(port: &Arc<dyn HyprlandPort>) -> Option<bareness::Bareness> {
    let port = Arc::clone(port);

    let answered = tokio::task::spawn_blocking(move || {
        let snapshot = port.workspace_snapshot()?;
        let clients = port.clients_snapshot()?;

        Ok::<_, hydebar_proto::ports::hyprland::HyprlandError>((snapshot, clients))
    })
    .await;

    match answered {
        Ok(Ok((snapshot, clients))) => Some(bareness::read(&snapshot, &clients)),
        Ok(Err(err)) => {
            error!("failed to read the screens: {err}");
            None
        }
        Err(err) => {
            error!("the screen reading task failed: {err}");
            None
        }
    }
}
