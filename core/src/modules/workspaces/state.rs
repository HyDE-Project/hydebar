//! Message folding and command dispatch for the workspaces module.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandMonitorSelector, HyprlandWorkspaceSelector};
use log::{debug, error};

use super::{Message, Workspaces, snapshot::map_snapshot_to_workspaces};
use crate::config::WorkspacesModuleConfig;

impl Workspaces {
    pub fn update(&mut self, message: Message, config: &WorkspacesModuleConfig) {
        match message {
            Message::WorkspacesChanged(snapshot) => {
                self.items = map_snapshot_to_workspaces(&snapshot, config);
                self.urgent_ids
                    .retain(|id| self.items.iter().any(|w| w.id == *id && !w.active));

                for workspace in &mut self.items {
                    workspace.urgent = self.urgent_ids.contains(&workspace.id);
                }
            }
            Message::WorkspaceUrgent(id) => {
                let visible = self.items.iter().any(|w| w.id == id && w.active);

                if !visible {
                    self.urgent_ids.insert(id);

                    if let Some(workspace) = self.items.iter_mut().find(|w| w.id == id) {
                        workspace.urgent = true;
                    }
                }
            }
            Message::ChangeWorkspace(id) => {
                if id > 0 {
                    let already_active = self.items.iter().any(|w| w.active && w.id == id);
                    if !already_active {
                        debug!("changing workspace to: {id}");
                        let port = Arc::clone(&self.hyprland);
                        self.spawn_dispatch(move || {
                            port.change_workspace(HyprlandWorkspaceSelector::Id(id))
                        });
                    }
                }
            }
            Message::ToggleSpecialWorkspace(id) => {
                if let Some(special) = self.items.iter().find(|w| w.id == id && w.id < 0) {
                    debug!("toggle special workspace: {id}");

                    let monitor_ident = special.monitor_id.map_or_else(
                        || HyprlandMonitorSelector::Name(special.monitor.clone()),
                        HyprlandMonitorSelector::Id
                    );
                    let name = special.name.clone();
                    let port = Arc::clone(&self.hyprland);

                    self.spawn_dispatch(move || {
                        port.focus_and_toggle_special_workspace(monitor_ident, &name)
                    });
                }
            }
        }
    }

    /// Runs a compositor dispatch off the thread the bar draws on.
    ///
    /// The port retries with a timeout when the compositor socket stalls;
    /// waiting that out on the update thread would freeze every module. A
    /// module that was never registered has no runtime and dispatches inline,
    /// which keeps tests synchronous.
    fn spawn_dispatch(
        &self,
        dispatch: impl FnOnce() -> Result<(), hydebar_proto::ports::hyprland::HyprlandError>
        + Send
        + 'static
    ) {
        match &self.runtime {
            Some(runtime) => {
                runtime.spawn_blocking(move || {
                    if let Err(err) = dispatch() {
                        error!("failed to dispatch workspace command: {err}");
                    }
                });
            }
            None => {
                if let Err(err) = dispatch() {
                    error!("failed to dispatch workspace command: {err}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hydebar_proto::{config::WorkspacesModuleConfig, ports::hyprland::HyprlandPort};

    use super::{Message, Workspaces};
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_from_port_snapshot() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port;
        let config = WorkspacesModuleConfig::default();

        let module = Workspaces::new(port_trait, &config);

        assert!(!module.items().is_empty());
    }

    #[test]
    fn change_workspace_dispatches_via_port() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let config = WorkspacesModuleConfig::default();

        let mut module = Workspaces::new(port_trait, &config);
        module.update(Message::ChangeWorkspace(2), &config);

        assert_eq!(port.workspace_calls(), 1);
    }
}
