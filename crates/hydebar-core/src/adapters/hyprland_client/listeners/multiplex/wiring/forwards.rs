//! One-to-one forwards: a raw event restated as exactly one port event.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandWindowEvent, HyprlandWorkspaceEvent};
use hyprland::event_listener::AsyncEventListener;

use super::fan_out;
use crate::adapters::hyprland_client::listeners::multiplex::singleton::Multiplexer;

/// Registers every handler that forwards a single event verbatim.
pub(super) fn register(listener: &mut AsyncEventListener, mux: &Arc<Multiplexer>) {
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
}
