//! Translation of raw compositor records into the port's snapshot types.
//!
//! The compositor's own records carry its conventions — a zero workspace id
//! standing for none, monitor ids wider than the port speaks, a focus order
//! whose head is the focused window — and this is the one place those
//! conventions are restated in the port's terms.

use hydebar_proto::ports::hyprland::{
    HyprlandClientInfo, HyprlandMonitorInfo, HyprlandWorkspaceInfo
};

use crate::adapters::compositor::records::{Client, Monitor, Workspace};

/// Restates one compositor monitor record in the port's terms.
pub(super) fn monitor_info(monitor: Monitor) -> HyprlandMonitorInfo {
    HyprlandMonitorInfo {
        id:                   i32::try_from(monitor.id).unwrap_or(i32::MAX),
        name:                 monitor.name,
        active_workspace_id:  (monitor.active_workspace.id != 0)
            .then_some(monitor.active_workspace.id),
        special_workspace_id: (monitor.special_workspace.id != 0)
            .then_some(monitor.special_workspace.id)
    }
}

/// Restates one compositor workspace record in the port's terms.
pub(super) fn workspace_info(workspace: Workspace) -> HyprlandWorkspaceInfo {
    HyprlandWorkspaceInfo {
        id:           workspace.id,
        name:         workspace.name,
        monitor_id:   workspace
            .monitor_id
            .and_then(|monitor_id| usize::try_from(monitor_id).ok()),
        monitor_name: workspace.monitor,
        window_count: workspace.windows
    }
}

/// Restates one compositor client record in the port's terms.
pub(super) fn client_info(client: Client) -> HyprlandClientInfo {
    HyprlandClientInfo {
        address:      client.address,
        class:        client.class,
        title:        client.title,
        workspace_id: client.workspace.id,
        focused:      client.focus_history_id == 0,
        floating:     client.floating,
        at:           (i32::from(client.at.0), i32::from(client.at.1)),
        size:         (i32::from(client.size.0), i32::from(client.size.1))
    }
}
