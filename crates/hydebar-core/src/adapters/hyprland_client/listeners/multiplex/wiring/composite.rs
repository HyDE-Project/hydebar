//! Handlers that answer one raw event with several port events, or with a
//! compositor question first.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{
    HyprlandKeyboardEvent, HyprlandPort, HyprlandWindowEvent, HyprlandWorkspaceEvent
};
use hyprland::{event_listener::AsyncEventListener, shared::HyprData};

use super::fan_out;
use crate::adapters::hyprland_client::{
    HyprlandClient, listeners::multiplex::singleton::Multiplexer
};

/// Registers every handler that fans out more than one event or asks the
/// compositor before answering.
pub(super) fn register(
    listener: &mut AsyncEventListener,
    mux: &Arc<Multiplexer>,
    client: &HyprlandClient
) {
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
}
