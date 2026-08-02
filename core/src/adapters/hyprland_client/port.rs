//! The [`HyprlandPort`] implementation: every operation the bar asks of the
//! compositor.
//!
//! Event subscriptions tap the multiplexed connection, snapshots and commands
//! go through the retry policy of the client's configuration, and every raw
//! record crossing the boundary is restated through [`super::snapshot`].

use hydebar_proto::ports::hyprland::{
    HyprlandClientInfo, HyprlandError, HyprlandEventStream, HyprlandKeyboardEvent,
    HyprlandKeyboardState, HyprlandMonitorSelector, HyprlandPort, HyprlandWindowEvent,
    HyprlandWindowInfo, HyprlandWorkspaceEvent, HyprlandWorkspaceSelector,
    HyprlandWorkspaceSnapshot
};
use hyprland::{
    ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes,
    data::{Client, Clients, Devices, Monitors, Workspace, Workspaces},
    keyword::Keyword,
    shared::{HyprData, HyprDataActive, HyprDataActiveOptional}
};

use super::{HyprlandClient, dispatch, listeners::multiplex, snapshot};

const WORKSPACE_SNAPSHOT_OP: &str = "workspace_snapshot";
const ACTIVE_WINDOW_OP: &str = "active_window";
const CHANGE_WORKSPACE_OP: &str = "change_workspace";
const TOGGLE_SPECIAL_OP: &str = "toggle_special_workspace";
const KEYBOARD_STATE_OP: &str = "keyboard_state";
const SWITCH_LAYOUT_OP: &str = "switch_keyboard_layout";
const CLIENTS_SNAPSHOT_OP: &str = "clients_snapshot";
const FOCUS_WINDOW_OP: &str = "focus_window";

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

            let monitors = monitors.into_iter().map(snapshot::monitor_info).collect();

            let workspaces = workspaces
                .into_iter()
                .map(snapshot::workspace_info)
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

    fn clients_snapshot(&self) -> Result<Vec<HyprlandClientInfo>, HyprlandError> {
        self.execute_with_retry(CLIENTS_SNAPSHOT_OP, || {
            let clients =
                Clients::get().map_err(|err| Self::backend_error(CLIENTS_SNAPSHOT_OP, err))?;

            Ok(clients
                .into_iter()
                .filter(|client| client.mapped)
                .map(snapshot::client_info)
                .collect())
        })
    }

    fn focus_window(&self, address: &str) -> Result<(), HyprlandError> {
        let address = address.to_string();
        self.execute_with_retry(FOCUS_WINDOW_OP, move || {
            dispatch::dispatch_in_any_dialect(|dialect| dispatch::focus_window(dialect, &address))
                .map_err(|err| Self::backend_error(FOCUS_WINDOW_OP, err))
        })
    }
}
