//! Restating one compositor announcement as the port events it stands for.
//!
//! One connection carries every domain, so this is where each announcement is
//! answered with the port events it means and fanned out to the matching
//! broadcast channels. Nobody listening on a channel is fine — a tap nobody
//! holds simply drops what it is sent.
//!
//! Two announcements cannot be answered from the line alone. A window asking
//! for attention names itself by address, and the workspace the readout marks
//! has to be looked up; a keyboard layout change names the keyboard, and the
//! layout now in force has to be asked for. Both go to the blocking pool, so
//! the round trip does not hold up the socket the rest arrive on.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{
    HyprlandKeyboardEvent, HyprlandPort, HyprlandWindowEvent, HyprlandWorkspaceEvent
};
use tokio::sync::broadcast;

use super::singleton::Multiplexer;
use crate::adapters::{
    compositor::{events::CompositorEvent, query},
    hyprland_client::HyprlandClient
};

/// The name a lookup made while answering an announcement reports under.
const URGENT_LOOKUP_OP: &str = "urgent_workspace";

/// Sends an event to whoever is listening; nobody listening is fine.
fn fan_out<T>(tx: &broadcast::Sender<T>, event: T) {
    let _ = tx.send(event);
}

/// Answers one announcement with every port event it stands for.
pub(super) async fn dispatch(
    event: CompositorEvent,
    mux: &Arc<Multiplexer>,
    client: &HyprlandClient
) {
    match event {
        CompositorEvent::WorkspaceChanged => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::Changed);
            fan_out(&mux.window, HyprlandWindowEvent::WorkspaceFocusChanged);
        }
        CompositorEvent::WindowClosed => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::WindowClosed);
            fan_out(&mux.window, HyprlandWindowEvent::WindowClosed);
        }
        CompositorEvent::WorkspaceAdded => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::Added);
        }
        CompositorEvent::WorkspaceRemoved => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::Removed);
        }
        CompositorEvent::WorkspaceMoved => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::Moved);
        }
        CompositorEvent::SpecialChanged => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::SpecialChanged);
        }
        CompositorEvent::SpecialRemoved => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::SpecialRemoved);
        }
        CompositorEvent::MonitorRemoved => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::Changed);
        }
        CompositorEvent::WindowOpened => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::WindowOpened);
        }
        CompositorEvent::WindowMoved => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::WindowMoved);
        }
        CompositorEvent::ActiveMonitorChanged => {
            fan_out(&mux.workspace, HyprlandWorkspaceEvent::ActiveMonitorChanged);
        }
        CompositorEvent::ActiveWindowChanged => {
            fan_out(&mux.window, HyprlandWindowEvent::ActiveWindowChanged);
        }
        CompositorEvent::WindowTitleChanged => {
            fan_out(&mux.window, HyprlandWindowEvent::WindowTitleChanged);
        }
        CompositorEvent::SubmapChanged(submap) => {
            fan_out(&mux.keyboard, HyprlandKeyboardEvent::SubmapChanged(submap));
        }
        CompositorEvent::Urgent {
            address
        } => mark_urgent(mux, address).await,
        CompositorEvent::LayoutChanged => announce_layout(mux, client).await,
        CompositorEvent::ConfigReloaded => {
            fan_out(&mux.reload, ());
            announce_layout_count(mux, client).await;
        }
    }
}

/// Marks the workspace holding the window that asked for attention.
///
/// The announcement names the window and nothing else, so the workspace is
/// looked up; a window already gone by the time the answer arrives marks
/// nothing, which is what the reader would have wanted anyway.
async fn mark_urgent(mux: &Arc<Multiplexer>, address: String) {
    let workspace_id = tokio::task::spawn_blocking(move || {
        query::clients(URGENT_LOOKUP_OP).ok().and_then(|clients| {
            clients
                .into_iter()
                .find(|client| client.address == address)
                .map(|client| client.workspace.id)
        })
    })
    .await
    .ok()
    .flatten();

    if let Some(workspace_id) = workspace_id {
        fan_out(
            &mux.workspace,
            HyprlandWorkspaceEvent::Urgent {
                workspace_id
            }
        );
    }
}

/// Announces the layout the keyboard is now in.
async fn announce_layout(mux: &Arc<Multiplexer>, client: &HyprlandClient) {
    if let Some(state) = keyboard_state(client).await {
        fan_out(
            &mux.keyboard,
            HyprlandKeyboardEvent::LayoutChanged(state.active_layout)
        );
    }
}

/// Announces whether the keyboard still has more than one layout to step to.
async fn announce_layout_count(mux: &Arc<Multiplexer>, client: &HyprlandClient) {
    if let Some(state) = keyboard_state(client).await {
        fan_out(
            &mux.keyboard,
            HyprlandKeyboardEvent::LayoutConfigurationChanged(state.has_multiple_layouts)
        );
    }
}

/// Asks the compositor what the keyboard is doing, off the socket thread.
async fn keyboard_state(
    client: &HyprlandClient
) -> Option<hydebar_proto::ports::hyprland::HyprlandKeyboardState> {
    let client = client.clone();

    tokio::task::spawn_blocking(move || client.keyboard_state().ok())
        .await
        .ok()
        .flatten()
}
