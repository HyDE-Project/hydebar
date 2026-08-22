//! Configuration for the workspaces module.

use serde::Deserialize;

/// Which workspaces are rendered on a given output.
#[derive(Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub enum WorkspaceVisibilityMode {
    #[default]
    /// Every workspace the compositor holds, on every screen.
    All,
    /// Only the workspaces belonging to the screen being drawn.
    MonitorSpecific
}

/// Workspace module behaviour.
#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct WorkspacesModuleConfig {
    #[serde(default)]
    /// Which workspaces a screen shows.
    pub visibility_mode:          WorkspaceVisibilityMode,
    #[serde(default)]
    /// Whether the gaps between numbers are drawn as empty workspaces.
    pub enable_workspace_filling: bool,
    /// How many workspaces are drawn at most.
    pub max_workspaces:           Option<u32>
}
