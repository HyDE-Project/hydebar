//! Events the Hyprland port streams to its subscribers.

/// Events related to Hyprland windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandWindowEvent {
    /// The active window changed.
    ActiveWindowChanged,
    /// A window changed its title in place, focus untouched.
    WindowTitleChanged,
    /// A workspace focus change occurred.
    WorkspaceFocusChanged,
    /// A window was closed.
    WindowClosed
}

/// Events related to Hyprland workspaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandWorkspaceEvent {
    /// A new workspace was added.
    Added,
    /// Workspace metadata changed.
    Changed,
    /// A workspace was removed.
    Removed,
    /// A workspace was moved to another monitor.
    Moved,
    /// The active special workspace changed.
    SpecialChanged,
    /// A special workspace was removed.
    SpecialRemoved,
    /// A window opened within a workspace.
    WindowOpened,
    /// A window closed within a workspace.
    WindowClosed,
    /// A window was moved between workspaces.
    WindowMoved,
    /// The active monitor changed.
    ActiveMonitorChanged,
    /// A window on the workspace demanded attention.
    Urgent {
        /// Workspace holding the demanding window.
        workspace_id: i32
    }
}

/// Keyboard related Hyprland events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandKeyboardEvent {
    /// The active keyboard layout changed.
    LayoutChanged(String),
    /// Keyboard layout configuration changed (e.g. config reload).
    LayoutConfigurationChanged(bool),
    /// The active keyboard submap changed.
    SubmapChanged(Option<String>)
}
