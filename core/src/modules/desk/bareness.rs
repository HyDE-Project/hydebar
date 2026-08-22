//! Which screens have nothing on them, read off a compositor snapshot.
//!
//! The desk unfolds on a screen no window has taken, so the question is asked
//! per screen rather than per session: a second monitor running a browser must
//! keep its strip while the first one, cleared, gets the whole wallpaper.
//!
//! Taken, not merely touched: a window that tiles into the workspace has been
//! given the screen, and a window that floats over it has not. A calculator, a
//! dialog, a picture in picture sits above whatever is there and leaves the
//! screen to it, so it is not what folds the desk away.

use hydebar_proto::ports::hyprland::{HyprlandClientInfo, HyprlandWorkspaceSnapshot};

/// The screens the desk may unfold on.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Bareness {
    /// Names of the monitors whose visible workspace holds no tiled window.
    screens: Vec<String>,
    /// Whether the focused workspace holds no tiled window.
    ///
    /// The bar runs on a nameless fallback surface until the compositor
    /// reports its monitors, and that surface covers whatever screen has the
    /// focus.
    focused: bool
}

impl Bareness {
    /// Reports whether the desk may unfold on the surface of `monitor`.
    ///
    /// A surface with no monitor name is the fallback one and answers for the
    /// focused screen.
    #[must_use]
    pub fn covers(&self, monitor: Option<&str>) -> bool {
        monitor.map_or(self.focused, |name| {
            self.screens.iter().any(|screen| screen == name)
        })
    }

    /// Reports whether `self` leaves more of the desktop bare than `other`.
    ///
    /// What the desk settles for before it unfolds: a window closing hands a
    /// screen over, and a screen handed over in the middle of a burst — the
    /// last window of a workspace closing as the next one maps — must not
    /// flash the whole canvas for a frame.
    #[must_use]
    pub fn unfolds_further_than(&self, other: &Self) -> bool {
        (self.focused && !other.focused)
            || self
                .screens
                .iter()
                .any(|screen| !other.screens.iter().any(|seen| seen == screen))
    }
}

/// Reports whether any client has tiled into the workspace of `id`.
fn taken(clients: &[HyprlandClientInfo], id: i32) -> bool {
    clients
        .iter()
        .any(|client| !client.floating && client.workspace_id == id)
}

/// Reads off a snapshot and a client list which screens hold no window.
///
/// A special workspace pulled up over a monitor is asked the same question as
/// the workspace under it: a drawer holding a tiled window has taken the
/// screen, and one holding nothing but a floating scratchpad has not.
#[must_use]
pub fn read(snapshot: &HyprlandWorkspaceSnapshot, clients: &[HyprlandClientInfo]) -> Bareness {
    let free = |id: i32| !taken(clients, id);

    let screens = snapshot
        .monitors
        .iter()
        .filter(|monitor| monitor.special_workspace_id.is_none_or(free))
        .filter(|monitor| monitor.active_workspace_id.is_some_and(free))
        .map(|monitor| monitor.name.clone())
        .collect();

    Bareness {
        screens,
        focused: snapshot.active_workspace_id.is_some_and(free)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_proto::ports::hyprland::{
        HyprlandMonitorInfo, HyprlandWorkspaceInfo, HyprlandWorkspaceSnapshot
    };

    use super::*;

    fn client(workspace_id: i32, floating: bool) -> HyprlandClientInfo {
        HyprlandClientInfo {
            address: format!("0x{workspace_id}{floating}"),
            class: "kitty".to_owned(),
            title: "shell".to_owned(),
            workspace_id,
            focused: false,
            floating,
            at: (0, 0),
            size: (100, 100)
        }
    }

    fn monitor(name: &str, active: i32) -> HyprlandMonitorInfo {
        HyprlandMonitorInfo {
            id:                   0,
            name:                 name.to_owned(),
            active_workspace_id:  Some(active),
            special_workspace_id: None
        }
    }

    fn snapshot(
        monitors: Vec<HyprlandMonitorInfo>,
        active: Option<i32>
    ) -> HyprlandWorkspaceSnapshot {
        HyprlandWorkspaceSnapshot {
            monitors,
            workspaces: Vec::<HyprlandWorkspaceInfo>::new(),
            active_workspace_id: active
        }
    }

    #[test]
    fn a_cleared_screen_is_handed_to_the_desk() {
        let state = read(&snapshot(vec![monitor("DP-1", 1)], Some(1)), &[]);

        assert!(state.covers(Some("DP-1")));
        assert!(state.covers(None));
    }

    #[test]
    fn one_tiled_window_is_enough_to_keep_the_strip() {
        let state = read(
            &snapshot(vec![monitor("DP-1", 1)], Some(1)),
            &[client(1, false)]
        );

        assert!(!state.covers(Some("DP-1")));
        assert!(!state.covers(None));
    }

    #[test]
    fn a_floating_window_is_a_visitor_and_never_folds_the_desk() {
        let state = read(
            &snapshot(vec![monitor("DP-1", 1)], Some(1)),
            &[client(1, true), client(1, true)]
        );

        assert!(state.covers(Some("DP-1")), "floats leave the screen bare");
        assert!(state.covers(None));
    }

    #[test]
    fn a_float_beside_a_tiled_window_still_leaves_the_strip() {
        let state = read(
            &snapshot(vec![monitor("DP-1", 1)], Some(1)),
            &[client(1, true), client(1, false)]
        );

        assert!(!state.covers(Some("DP-1")));
    }

    #[test]
    fn a_window_on_another_workspace_leaves_this_one_bare() {
        let state = read(
            &snapshot(vec![monitor("DP-1", 1)], Some(1)),
            &[client(2, false)]
        );

        assert!(state.covers(Some("DP-1")));
    }

    #[test]
    fn every_screen_answers_for_itself() {
        let state = read(
            &snapshot(vec![monitor("DP-1", 1), monitor("HDMI-A-1", 2)], Some(2)),
            &[client(2, false)]
        );

        assert!(state.covers(Some("DP-1")));
        assert!(!state.covers(Some("HDMI-A-1")));
        assert!(!state.covers(None));
    }

    #[test]
    fn a_drawer_holding_a_tiled_window_keeps_the_desk_folded() {
        let mut screen = monitor("DP-1", 1);
        screen.special_workspace_id = Some(-99);

        let state = read(&snapshot(vec![screen], Some(1)), &[client(-99, false)]);

        assert!(!state.covers(Some("DP-1")));
    }

    #[test]
    fn a_drawer_holding_only_a_float_leaves_the_desk_out() {
        let mut screen = monitor("DP-1", 1);
        screen.special_workspace_id = Some(-99);

        let state = read(&snapshot(vec![screen], Some(1)), &[client(-99, true)]);

        assert!(state.covers(Some("DP-1")));
    }

    #[test]
    fn a_screen_that_just_cleared_is_what_the_desk_settles_for() {
        let folded = read(
            &snapshot(vec![monitor("DP-1", 1)], Some(1)),
            &[client(1, false)]
        );
        let bare = read(&snapshot(vec![monitor("DP-1", 1)], Some(1)), &[]);

        assert!(bare.unfolds_further_than(&folded));
        assert!(!folded.unfolds_further_than(&bare));
        assert!(!bare.unfolds_further_than(&bare));
    }
}
