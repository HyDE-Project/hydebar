//! From compositor snapshot to the indicators the bar draws.

use hydebar_proto::ports::hyprland::{HyprlandPort, HyprlandWorkspaceSnapshot};
use itertools::Itertools;
use log::error;

use crate::config::WorkspacesModuleConfig;

/// One workspace indicator as the bar knows it.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub id:         i32,
    pub name:       String,
    /// Index for color lookup; may be `None`.
    pub monitor_id: Option<usize>,
    /// Monitor name for fallback.
    pub monitor:    String,
    pub active:     bool,
    pub urgent:     bool,
    pub windows:    u16
}

/// Asks the port for a snapshot and maps it, empty on failure.
pub(super) fn get_workspaces(
    port: &dyn HyprlandPort,
    config: &WorkspacesModuleConfig
) -> Vec<Workspace> {
    let snapshot = match port.workspace_snapshot() {
        Ok(snapshot) => snapshot,
        Err(err) => {
            error!("failed to retrieve workspace snapshot: {err}");
            return Vec::new();
        }
    };

    map_snapshot_to_workspaces(&snapshot, config)
}

/// Maps a compositor snapshot onto the indicators the bar draws.
///
/// A workspace is drawn active when its own monitor shows it, so every
/// monitor of a multi-head setup highlights its current workspace rather
/// than only the one holding the focus. A snapshot whose monitors report no
/// active workspace falls back to the single focused one, which is what a
/// minimal test double provides.
///
/// Workspaces are deduplicated by ID to avoid duplicates from Hyprland,
/// special workspaces keep only the tail of their `special:` name, and when
/// workspace filling is enabled the missing IDs up to the highest one seen
/// (or the configured maximum) are synthesized as empty indicators.
pub(super) fn map_snapshot_to_workspaces(
    snapshot: &HyprlandWorkspaceSnapshot,
    config: &WorkspacesModuleConfig
) -> Vec<Workspace> {
    let active = snapshot.active_workspace_id;
    let monitors = &snapshot.monitors;

    let workspaces: Vec<_> = snapshot.workspaces.iter().unique_by(|w| w.id).collect();

    let mut result: Vec<Workspace> = Vec::with_capacity(workspaces.len());

    let (special, normal): (Vec<_>, Vec<_>) = workspaces.into_iter().partition(|w| w.id < 0);

    for w in &special {
        result.push(Workspace {
            id:         w.id,
            name:       w
                .name
                .as_str()
                .split(':')
                .next_back()
                .map_or_else(String::new, ToOwned::to_owned),
            monitor_id: w.monitor_id,
            monitor:    w.monitor_name.clone(),
            active:     monitors
                .iter()
                .any(|m| m.special_workspace_id == Some(w.id)),
            urgent:     false,
            windows:    w.window_count
        });
    }

    let any_monitor_reports = monitors.iter().any(|m| m.active_workspace_id.is_some());

    for w in &normal {
        let shown = if any_monitor_reports {
            monitors.iter().any(|m| m.active_workspace_id == Some(w.id))
        } else {
            Some(w.id) == active
        };

        result.push(Workspace {
            id:         w.id,
            name:       w.name.clone(),
            monitor_id: w.monitor_id,
            monitor:    w.monitor_name.clone(),
            active:     shown,
            urgent:     false,
            windows:    w.window_count
        });
    }

    if !config.enable_workspace_filling || normal.is_empty() {
        result.sort_by_key(|w| w.id);
        return result;
    }

    let existing_ids = normal
        .iter()
        .map(|w| w.id)
        .collect::<std::collections::HashSet<_>>();
    let mut max_id = *existing_ids.iter().max().unwrap_or(&0);
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap,
        reason = "normal workspace ids are positive and configured maxima are far below i32::MAX"
    )]
    if let Some(max_workspaces) = config.max_workspaces
        && max_workspaces > max_id as u32
    {
        max_id = max_workspaces as i32;
    }

    let missing_ids: Vec<i32> = (1..=max_id)
        .filter(|id| !existing_ids.contains(id))
        .collect();

    result.reserve(missing_ids.len());

    for id in missing_ids {
        result.push(Workspace {
            id,
            name: id.to_string(),
            monitor_id: None,
            monitor: String::new(),
            active: false,
            urgent: false,
            windows: 0
        });
    }

    result.sort_by_key(|w| w.id);
    result
}
