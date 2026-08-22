//! Data carried across the Hyprland port: snapshots, metadata and the
//! selectors naming a monitor or workspace in dispatch calls.

use std::fmt;

/// Immutable snapshot describing monitors and workspaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandWorkspaceSnapshot {
    /// Known monitors reported by Hyprland.
    pub monitors:            Vec<HyprlandMonitorInfo>,
    /// Known workspaces reported by Hyprland.
    pub workspaces:          Vec<HyprlandWorkspaceInfo>,
    /// Identifier of the currently active workspace, if available.
    pub active_workspace_id: Option<i32>
}

/// Metadata describing a Hyprland monitor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandMonitorInfo {
    /// Monitor identifier as reported by Hyprland.
    pub id:                   i32,
    /// Human readable monitor name.
    pub name:                 String,
    /// ID of the workspace shown on this monitor, if known.
    pub active_workspace_id:  Option<i32>,
    /// ID of the special workspace focused on this monitor, if any.
    pub special_workspace_id: Option<i32>
}

/// Metadata describing a Hyprland workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandWorkspaceInfo {
    /// Workspace identifier.
    pub id:           i32,
    /// Workspace name.
    pub name:         String,
    /// Index of the monitor the workspace is assigned to, if any.
    pub monitor_id:   Option<usize>,
    /// Name of the monitor the workspace is assigned to.
    pub monitor_name: String,
    /// Number of windows currently present in the workspace.
    pub window_count: u16
}

/// Metadata describing the focused Hyprland window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandWindowInfo {
    /// Window title provided by the client.
    pub title: String,
    /// Window class name.
    pub class: String
}

/// One mapped window of the compositor's client list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandClientInfo {
    /// Compositor address uniquely naming the window.
    pub address:      String,
    /// Window class name.
    pub class:        String,
    /// Window title provided by the client.
    pub title:        String,
    /// Identifier of the workspace hosting the window.
    pub workspace_id: i32,
    /// Whether the window holds the focus.
    pub focused:      bool,
    /// Top left corner of the window on the compositor's own plane.
    ///
    /// The plane spans every screen, so a window on the second monitor stands
    /// beyond the first one's width. A reader drawing a miniature of a screen
    /// takes the monitor's own origin off it first.
    pub at:           (i32, i32),
    /// Width and height of the window, in the same plane.
    pub size:         (i32, i32),
    /// Whether the window floats over the layout instead of tiling into it.
    ///
    /// A floating window is a visitor: a dialog, a calculator, a picture in
    /// picture. It sits over whatever the workspace holds without taking the
    /// screen from it, which is why it is not what decides that a screen is
    /// occupied.
    pub floating:     bool
}

/// Snapshot of the keyboard state known to Hyprland.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandKeyboardState {
    /// Currently active XKB layout.
    pub active_layout:        String,
    /// Whether multiple layouts are configured.
    pub has_multiple_layouts: bool,
    /// Name of the currently active submap, if any.
    pub active_submap:        Option<String>
}

/// Identifies a monitor for Hyprland dispatch calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandMonitorSelector {
    /// Select monitor by its numeric identifier.
    Id(usize),
    /// Select monitor by its name.
    Name(String)
}

impl fmt::Display for HyprlandMonitorSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(f, "monitor-id:{id}"),
            Self::Name(name) => write!(f, "monitor-name:{name}")
        }
    }
}

/// Identifies a workspace for Hyprland dispatch calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandWorkspaceSelector {
    /// Select workspace by numeric identifier.
    Id(i32),
    /// Select workspace by name.
    Name(String)
}

impl fmt::Display for HyprlandWorkspaceSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(f, "workspace-id:{id}"),
            Self::Name(name) => write!(f, "workspace-name:{name}")
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn monitor_selector_display() {
        assert_eq!(HyprlandMonitorSelector::Id(3).to_string(), "monitor-id:3");
        assert_eq!(
            HyprlandMonitorSelector::Name("DP-1".into()).to_string(),
            "monitor-name:DP-1"
        );
    }

    #[test]
    fn workspace_selector_display() {
        assert_eq!(
            HyprlandWorkspaceSelector::Id(2).to_string(),
            "workspace-id:2"
        );
        assert_eq!(
            HyprlandWorkspaceSelector::Name("code".into()).to_string(),
            "workspace-name:code"
        );
    }

    #[test]
    fn keyboard_state_equality() {
        let state_a = HyprlandKeyboardState {
            active_layout:        "us".into(),
            has_multiple_layouts: true,
            active_submap:        Some("resize".into())
        };
        let state_b = state_a.clone();
        assert_eq!(state_a, state_b);
    }
}
