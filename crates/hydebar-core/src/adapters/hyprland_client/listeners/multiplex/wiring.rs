//! Registration of every compositor event handler on a fresh connection.
//!
//! One connection carries every domain, so this is where each raw compositor
//! event is restated as the port event it stands for and fanned out to the
//! matching broadcast channel. Nobody listening on a channel is fine — a tap
//! nobody holds simply drops what it is sent.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{
    HyprlandKeyboardEvent, HyprlandPort, HyprlandWindowEvent, HyprlandWorkspaceEvent
};
use hyprland::{event_listener::AsyncEventListener, shared::HyprData};
use tokio::sync::broadcast;

use super::singleton::Multiplexer;
use crate::adapters::hyprland_client::HyprlandClient;

/// Sends an event to whoever is listening; nobody listening is fine.
fn fan_out<T>(tx: &broadcast::Sender<T>, event: T) {
    let _ = tx.send(event);
}

/// Wires every handler of every domain onto one fresh connection.
#[expect(
    clippy::too_many_lines,
    reason = "declarative wiring of every Hyprland event handler; splitting would scatter the registrations"
)]
pub(super) fn build_listener(mux: &Arc<Multiplexer>, client: &HyprlandClient) -> AsyncEventListener {
    let mut listener = AsyncEventListener::new();

    macro_rules! forward_window {
        ($register:ident, $event:expr) => {
            listener.$register({
                let mux = Arc::clone(mux);
                move |_| {
                    let mux = Arc::clone(&mux);
                    Box::pin(async move {
                        fan_out(&mux.window, $event);
                    })
                }
            });
        };
    }

    forward_window!(
        add_active_window_changed_handler,
        HyprlandWindowEvent::ActiveWindowChanged
    );
    forward_window!(
        add_window_title_changed_handler,
        HyprlandWindowEvent::WindowTitleChanged
    );

    macro_rules! forward_workspace {
        ($register:ident, $event:expr) => {
            listener.$register({
                let mux = Arc::clone(mux);
                move |_| {
                    let mux = Arc::clone(&mux);
                    Box::pin(async move {
                        fan_out(&mux.workspace, $event);
                    })
                }
            });
        };
    }

    forward_workspace!(add_workspace_added_handler, HyprlandWorkspaceEvent::Added);
    forward_workspace!(
        add_workspace_deleted_handler,
        HyprlandWorkspaceEvent::Removed
    );
    forward_workspace!(add_workspace_moved_handler, HyprlandWorkspaceEvent::Moved);
    forward_workspace!(
        add_changed_special_handler,
        HyprlandWorkspaceEvent::SpecialChanged
    );
    forward_workspace!(
        add_special_removed_handler,
        HyprlandWorkspaceEvent::SpecialRemoved
    );
    forward_workspace!(add_monitor_removed_handler, HyprlandWorkspaceEvent::Changed);
    forward_workspace!(
        add_window_opened_handler,
        HyprlandWorkspaceEvent::WindowOpened
    );
    forward_workspace!(
        add_window_moved_handler,
        HyprlandWorkspaceEvent::WindowMoved
    );
    forward_workspace!(
        add_active_monitor_changed_handler,
        HyprlandWorkspaceEvent::ActiveMonitorChanged
    );

    listener.add_workspace_changed_handler({
        let mux = Arc::clone(mux);
        move |_| {
            let mux = Arc::clone(&mux);
            Box::pin(async move {
                fan_out(&mux.workspace, HyprlandWorkspaceEvent::Changed);
                fan_out(&mux.window, HyprlandWindowEvent::WorkspaceFocusChanged);
            })
        }
    });

    listener.add_window_closed_handler({
        let mux = Arc::clone(mux);
        move |_| {
            let mux = Arc::clone(&mux);
            Box::pin(async move {
                fan_out(&mux.workspace, HyprlandWorkspaceEvent::WindowClosed);
                fan_out(&mux.window, HyprlandWindowEvent::WindowClosed);
            })
        }
    });

    listener.add_urgent_state_changed_handler({
        let mux = Arc::clone(mux);
        move |address| {
            let mux = Arc::clone(&mux);
            Box::pin(async move {
                let workspace_id = tokio::task::spawn_blocking(move || {
                    hyprland::data::Clients::get().ok().and_then(|clients| {
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
            })
        }
    });

    listener.add_layout_changed_handler({
        let mux = Arc::clone(mux);
        let client = client.clone();
        move |_| {
            let mux = Arc::clone(&mux);
            let client = client.clone();
            Box::pin(async move {
                if let Ok(state) = client.keyboard_state() {
                    fan_out(
                        &mux.keyboard,
                        HyprlandKeyboardEvent::LayoutChanged(state.active_layout)
                    );
                }
            })
        }
    });

    listener.add_sub_map_changed_handler({
        let mux = Arc::clone(mux);
        move |submap| {
            let mux = Arc::clone(&mux);
            Box::pin(async move {
                let payload = if submap.trim().is_empty() {
                    None
                } else {
                    Some(submap)
                };

                fan_out(&mux.keyboard, HyprlandKeyboardEvent::SubmapChanged(payload));
            })
        }
    });

    listener.add_config_reloaded_handler({
        let mux = Arc::clone(mux);
        let client = client.clone();
        move || {
            let mux = Arc::clone(&mux);
            let client = client.clone();
            Box::pin(async move {
                fan_out(&mux.reload, ());

                if let Ok(state) = client.keyboard_state() {
                    fan_out(
                        &mux.keyboard,
                        HyprlandKeyboardEvent::LayoutConfigurationChanged(
                            state.has_multiple_layouts
                        )
                    );
                }
            })
        }
    });

    listener
}
