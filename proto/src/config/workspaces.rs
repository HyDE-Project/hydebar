//! Configuration for the workspaces module.

use serde::Deserialize;

/// Which workspaces are rendered on a given output.
#[derive(Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub enum WorkspaceVisibilityMode {
    #[default]
    All,
    MonitorSpecific
}

/// Workspace module behaviour.
#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct WorkspacesModuleConfig {
    #[serde(default)]
    pub visibility_mode:          WorkspaceVisibilityMode,
    #[serde(default)]
    pub enable_workspace_filling: bool,
    pub max_workspaces:           Option<u32>
}
