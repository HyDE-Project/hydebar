//! The workspace indicators: one button per workspace, live on every screen.
//!
//! One folder, five rooms: [`snapshot`] maps the compositor's state onto
//! indicators, [`listener`] follows its events in the background, [`state`]
//! folds messages in and dispatches commands, [`view`] draws the row and
//! [`module`] wires the module to the bar. The root holds the state the
//! rooms share.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandPort, HyprlandWorkspaceSnapshot};
use tokio::task::JoinHandle;

use crate::{ModuleEventSender, config::WorkspacesModuleConfig};

mod listener;
mod module;
mod snapshot;
mod state;
mod view;

pub use snapshot::Workspace;

#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesChanged(HyprlandWorkspaceSnapshot),
    /// A window on the workspace demanded attention.
    WorkspaceUrgent(i32),
    ChangeWorkspace(i32),
    ToggleSpecialWorkspace(i32)
}

pub struct Workspaces {
    hyprland:   Arc<dyn HyprlandPort>,
    items:      Vec<Workspace>,
    /// Workspaces whose windows demanded attention and were not yet visited.
    urgent_ids: std::collections::HashSet<i32>,
    sender:     Option<ModuleEventSender<Message>>,
    runtime:    Option<tokio::runtime::Handle>,
    task:       Option<JoinHandle<()>>
}

impl Workspaces {
    pub fn new(hyprland: Arc<dyn HyprlandPort>, config: &WorkspacesModuleConfig) -> Self {
        let workspaces = snapshot::get_workspaces(hyprland.as_ref(), config);

        Self {
            hyprland,
            items: workspaces,
            urgent_ids: std::collections::HashSet::new(),
            sender: None,
            runtime: None,
            task: None
        }
    }

    #[cfg(test)]
    pub(crate) fn items(&self) -> &[Workspace] {
        &self.items
    }
}

impl std::fmt::Debug for Workspaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspaces")
            .field("hyprland", &"<HyprlandPort>")
            .field("items", &self.items)
            .field("urgent_ids", &self.urgent_ids)
            .field("sender", &self.sender)
            .field("runtime", &self.runtime)
            .field("task", &self.task)
            .finish()
    }
}
