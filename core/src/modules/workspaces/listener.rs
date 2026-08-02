//! The background listener: compositor events in, snapshots out.

use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::{HyprlandEventStream, HyprlandPort, HyprlandWorkspaceEvent};
use log::error;
use tokio::time::sleep;
use tokio_stream::StreamExt;

use super::Message;
use crate::ModuleEventSender;

const WORKSPACE_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Follows the compositor's workspace events for as long as the module runs.
///
/// Publishes one snapshot up front, then one per settled burst of events; a
/// broken stream is reopened after a short delay rather than giving up.
pub(super) async fn run(hyprland: Arc<dyn HyprlandPort>, sender: ModuleEventSender<Message>) {
    publish_snapshot(&hyprland, &sender).await;

    loop {
        match hyprland.workspace_events() {
            Ok(mut stream) => {
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(
                            HyprlandWorkspaceEvent::Added
                            | HyprlandWorkspaceEvent::Changed
                            | HyprlandWorkspaceEvent::Removed
                            | HyprlandWorkspaceEvent::Moved
                            | HyprlandWorkspaceEvent::SpecialChanged
                            | HyprlandWorkspaceEvent::SpecialRemoved
                            | HyprlandWorkspaceEvent::WindowClosed
                            | HyprlandWorkspaceEvent::WindowOpened
                            | HyprlandWorkspaceEvent::WindowMoved
                            | HyprlandWorkspaceEvent::ActiveMonitorChanged
                        ) => {
                            let urgent = drain_burst(&mut stream).await;
                            publish_snapshot(&hyprland, &sender).await;

                            for id in urgent {
                                sender.send(Message::WorkspaceUrgent(id));
                            }
                        }
                        Ok(HyprlandWorkspaceEvent::Urgent {
                            workspace_id
                        }) => {
                            sender.send(Message::WorkspaceUrgent(workspace_id));
                        }
                        Err(err) => {
                            error!("workspace event stream error: {err}");
                        }
                    }
                }
            }
            Err(err) => {
                error!("failed to start workspace event stream: {err}");
            }
        }

        sleep(WORKSPACE_EVENT_RETRY_DELAY).await;
    }
}

/// Resolves a fresh snapshot off the async workers and publishes it.
///
/// The port call blocks on the compositor socket with retries, so it runs on
/// the blocking pool rather than on a runtime worker the other modules share.
async fn publish_snapshot(port: &Arc<dyn HyprlandPort>, sender: &ModuleEventSender<Message>) {
    let port = Arc::clone(port);

    match tokio::task::spawn_blocking(move || port.workspace_snapshot()).await {
        Ok(Ok(snapshot)) => {
            sender.send(Message::WorkspacesChanged(snapshot));
        }
        Ok(Err(err)) => error!("failed to retrieve workspace snapshot: {err}"),
        Err(err) => error!("workspace snapshot task failed: {err}")
    }
}

/// Swallows the rest of an event burst before the snapshot is taken.
///
/// A window being dragged across monitors lands as a handful of events within
/// a few milliseconds; one snapshot at the end of the burst shows the same
/// state as one per event, without the round-trips.
async fn drain_burst(stream: &mut HyprlandEventStream<HyprlandWorkspaceEvent>) -> Vec<i32> {
    const SETTLE: Duration = Duration::from_millis(25);

    let mut urgent = Vec::new();

    while let Ok(Some(event)) = tokio::time::timeout(SETTLE, stream.next()).await {
        if let Ok(HyprlandWorkspaceEvent::Urgent {
            workspace_id
        }) = event
        {
            urgent.push(workspace_id);
        }
    }

    urgent
}
