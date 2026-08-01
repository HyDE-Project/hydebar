mod config;
mod dispatch;
mod listeners;
mod sync_ops;
mod util;

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{
    HyprlandError, HyprlandEventStream, HyprlandKeyboardEvent, HyprlandKeyboardState,
    HyprlandMonitorInfo, HyprlandMonitorSelector, HyprlandPort, HyprlandWindowEvent,
    HyprlandWindowInfo, HyprlandWorkspaceEvent, HyprlandWorkspaceInfo, HyprlandWorkspaceSelector,
    HyprlandWorkspaceSnapshot
};
use hyprland::{
    ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes,
    data::{Client, Devices, Monitors, Workspace, Workspaces},
    keyword::Keyword,
    shared::{HyprData, HyprDataActive, HyprDataActiveOptional}
};

pub use self::config::HyprlandClientConfig;
use self::{listeners::multiplex, sync_ops::execute_with_retry};

const WORKSPACE_SNAPSHOT_OP: &str = "workspace_snapshot";
const ACTIVE_WINDOW_OP: &str = "active_window";
const CHANGE_WORKSPACE_OP: &str = "change_workspace";
const TOGGLE_SPECIAL_OP: &str = "toggle_special_workspace";
const KEYBOARD_STATE_OP: &str = "keyboard_state";
const SWITCH_LAYOUT_OP: &str = "switch_keyboard_layout";

/// [`HyprlandPort`] implementation backed by the `hyprland-rs` crate.
#[derive(Clone, Debug)]
pub struct HyprlandClient {
    config: Arc<HyprlandClientConfig>
}

impl Default for HyprlandClient {
    fn default() -> Self {
        Self {
            config: Arc::new(HyprlandClientConfig::default())
        }
    }
}

impl HyprlandClient {
    /// Construct a new [`HyprlandClient`] using
    /// [`HyprlandClientConfig::default`].
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a [`HyprlandClient`] with the provided configuration.
    #[must_use]
    pub fn with_config(config: HyprlandClientConfig) -> Self {
        Self {
            config: Arc::new(config)
        }
    }

    pub(crate) fn backend_error<E>(operation: &'static str, err: E) -> HyprlandError
    where
        E: std::error::Error + Send + Sync + 'static
    {
        HyprlandError::Backend {
            operation,
            source: Box::new(err)
        }
    }

    fn execute_with_retry<R, F>(
        &self,
        operation: &'static str,
        func: F
    ) -> Result<R, HyprlandError>
    where
        R: Send + 'static,
        F: Fn() -> Result<R, HyprlandError> + Send + Sync + 'static
    {
        execute_with_retry(&self.config, operation, func)
    }

    /// A blocking tap of the compositor's configuration reloads.
    #[must_use]
    pub fn config_reloads(&self) -> tokio::sync::broadcast::Receiver<()> {
        multiplex::config_reloads(self, &self.config)
    }
}

impl HyprlandPort for HyprlandClient {
    fn window_events(&self) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
        Ok(multiplex::window_events(self, &self.config))
    }

    fn workspace_events(
        &self
    ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
        Ok(multiplex::workspace_events(self, &self.config))
    }

    fn keyboard_events(
        &self
    ) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
        Ok(multiplex::keyboard_events(self, &self.config))
    }

    fn active_window(&self) -> Result<Option<HyprlandWindowInfo>, HyprlandError> {
        self.execute_with_retry(ACTIVE_WINDOW_OP, || {
            Client::get_active()
                .map_err(|err| Self::backend_error(ACTIVE_WINDOW_OP, err))
                .map(|maybe_client| {
                    maybe_client.map(|client| HyprlandWindowInfo {
                        title: client.title,
                        class: client.class
                    })
                })
        })
    }

    fn workspace_snapshot(&self) -> Result<HyprlandWorkspaceSnapshot, HyprlandError> {
        self.execute_with_retry(WORKSPACE_SNAPSHOT_OP, || {
            let monitors =
                Monitors::get().map_err(|err| Self::backend_error(WORKSPACE_SNAPSHOT_OP, err))?;
            let workspaces = Workspaces::get()
                .map_err(|err| Self::backend_error(WORKSPACE_SNAPSHOT_OP, err))?;
            let active = Workspace::get_active()
                .map_err(|err| Self::backend_error(WORKSPACE_SNAPSHOT_OP, err))?;

            let monitors = monitors
                .into_iter()
                .map(|monitor| HyprlandMonitorInfo {
                    id:                   i32::try_from(monitor.id).unwrap_or(i32::MAX),
                    name:                 monitor.name,
                    active_workspace_id:  (monitor.active_workspace.id != 0)
                        .then_some(monitor.active_workspace.id),
                    special_workspace_id: (monitor.special_workspace.id != 0)
                        .then_some(monitor.special_workspace.id)
                })
                .collect();

            let workspaces = workspaces
                .into_iter()
                .map(|workspace| HyprlandWorkspaceInfo {
                    id:           workspace.id,
                    name:         workspace.name,
                    monitor_id:   workspace
                        .monitor_id
                        .and_then(|monitor_id| usize::try_from(monitor_id).ok()),
                    monitor_name: workspace.monitor,
                    window_count: workspace.windows
                })
                .collect();

            Ok(HyprlandWorkspaceSnapshot {
                monitors,
                workspaces,
                active_workspace_id: Some(active.id)
            })
        })
    }

    fn change_workspace(&self, workspace: HyprlandWorkspaceSelector) -> Result<(), HyprlandError> {
        self.execute_with_retry(CHANGE_WORKSPACE_OP, move || {
            dispatch::dispatch_in_any_dialect(|dialect| {
                dispatch::focus_workspace(dialect, &workspace)
            })
            .map_err(|err| Self::backend_error(CHANGE_WORKSPACE_OP, err))
        })
    }

    fn focus_and_toggle_special_workspace(
        &self,
        monitor: HyprlandMonitorSelector,
        workspace_name: &str
    ) -> Result<(), HyprlandError> {
        let workspace_name = workspace_name.to_string();
        self.execute_with_retry(TOGGLE_SPECIAL_OP, move || {
            dispatch::dispatch_in_any_dialect(|dialect| dispatch::focus_monitor(dialect, &monitor))
                .and_then(|()| {
                    dispatch::dispatch_in_any_dialect(|dialect| {
                        dispatch::toggle_special_workspace(dialect, &workspace_name)
                    })
                })
                .map_err(|err| Self::backend_error(TOGGLE_SPECIAL_OP, err))
        })
    }

    fn keyboard_state(&self) -> Result<HyprlandKeyboardState, HyprlandError> {
        self.execute_with_retry(KEYBOARD_STATE_OP, || {
            let keyword = Keyword::get("input:kb_layout")
                .map_err(|err| Self::backend_error(KEYBOARD_STATE_OP, err))?;
            let has_multiple_layouts = keyword
                .value
                .to_string()
                .split(',')
                .filter(|value| !value.trim().is_empty())
                .count()
                > 1;

            let devices =
                Devices::get().map_err(|err| Self::backend_error(KEYBOARD_STATE_OP, err))?;
            let active_layout = devices
                .keyboards
                .iter()
                .find(|keyboard| keyboard.main)
                .map_or_else(
                    || "unknown".to_string(),
                    |keyboard| keyboard.active_keymap.clone()
                );

            Ok(HyprlandKeyboardState {
                active_layout,
                has_multiple_layouts,
                active_submap: None
            })
        })
    }

    fn switch_keyboard_layout(&self) -> Result<(), HyprlandError> {
        self.execute_with_retry(SWITCH_LAYOUT_OP, || {
            hyprland::ctl::switch_xkb_layout::call("all", SwitchXKBLayoutCmdTypes::Next)
                .map_err(|err| Self::backend_error(SWITCH_LAYOUT_OP, err))
        })
    }
}
