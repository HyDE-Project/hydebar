//! Which screens have nothing on them, read off a compositor snapshot.
//!
//! The desk unfolds on a screen showing no window at all, so the question is
//! asked per screen rather than per session: a second monitor running a
//! browser must keep its strip while the first one, cleared, gets the whole
//! wallpaper.

use hydebar_proto::ports::hyprland::HyprlandWorkspaceSnapshot;

/// The screens the desk may unfold on.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Bareness {
    /// Names of the monitors whose visible workspace holds no window.
    screens: Vec<String>,
    /// Whether the focused workspace holds no window.
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

/// Reads off a snapshot which of its screens hold no window.
///
/// A special workspace pulled up over a monitor counts as something on the
/// screen whatever it holds: it is a drawer the user opened, and the desk
/// must not paint over it. A monitor naming a workspace the snapshot does not
/// carry is left folded, because the count behind it is unknown rather than
/// zero.
#[must_use]
pub fn read(snapshot: &HyprlandWorkspaceSnapshot) -> Bareness {
    let bare_workspace = |id: i32| {
        snapshot
            .workspaces
            .iter()
            .find(|workspace| workspace.id == id)
            .is_some_and(|workspace| workspace.window_count == 0)
    };

    let screens = snapshot
        .monitors
        .iter()
        .filter(|monitor| monitor.special_workspace_id.is_none())
        .filter(|monitor| monitor.active_workspace_id.is_some_and(bare_workspace))
        .map(|monitor| monitor.name.clone())
        .collect();

    Bareness {
        screens,
        focused: snapshot.active_workspace_id.is_some_and(bare_workspace)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_proto::ports::hyprland::{HyprlandMonitorInfo, HyprlandWorkspaceInfo};

    use super::*;

    fn workspace(id: i32, monitor: &str, windows: u16) -> HyprlandWorkspaceInfo {
        HyprlandWorkspaceInfo {
            id,
            name: id.to_string(),
            monitor_id: Some(0),
            monitor_name: monitor.to_owned(),
            window_count: windows
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
        workspaces: Vec<HyprlandWorkspaceInfo>,
        active: Option<i32>
    ) -> HyprlandWorkspaceSnapshot {
        HyprlandWorkspaceSnapshot {
            monitors,
            workspaces,
            active_workspace_id: active
        }
    }

    #[test]
    fn a_cleared_screen_is_handed_to_the_desk() {
        let state = read(&snapshot(
            vec![monitor("DP-1", 1)],
            vec![workspace(1, "DP-1", 0)],
            Some(1)
        ));

        assert!(state.covers(Some("DP-1")));
        assert!(state.covers(None));
    }

    #[test]
    fn one_window_is_enough_to_keep_the_strip() {
        let state = read(&snapshot(
            vec![monitor("DP-1", 1)],
            vec![workspace(1, "DP-1", 1)],
            Some(1)
        ));

        assert!(!state.covers(Some("DP-1")));
        assert!(!state.covers(None));
    }

    #[test]
    fn every_screen_answers_for_itself() {
        let state = read(&snapshot(
            vec![monitor("DP-1", 1), monitor("HDMI-A-1", 2)],
            vec![workspace(1, "DP-1", 0), workspace(2, "HDMI-A-1", 3)],
            Some(2)
        ));

        assert!(state.covers(Some("DP-1")));
        assert!(!state.covers(Some("HDMI-A-1")));
        assert!(!state.covers(None));
    }

    #[test]
    fn a_special_workspace_pulled_up_keeps_the_desk_folded() {
        let mut screen = monitor("DP-1", 1);
        screen.special_workspace_id = Some(-99);

        let state = read(&snapshot(
            vec![screen],
            vec![workspace(1, "DP-1", 0)],
            Some(1)
        ));

        assert!(!state.covers(Some("DP-1")));
    }

    #[test]
    fn a_workspace_the_snapshot_never_mentions_is_left_folded() {
        let state = read(&snapshot(vec![monitor("DP-1", 7)], Vec::new(), Some(7)));

        assert!(!state.covers(Some("DP-1")));
        assert!(!state.covers(None));
    }

    #[test]
    fn a_screen_that_just_cleared_is_what_the_desk_settles_for() {
        let folded = read(&snapshot(
            vec![monitor("DP-1", 1)],
            vec![workspace(1, "DP-1", 1)],
            Some(1)
        ));
        let bare = read(&snapshot(
            vec![monitor("DP-1", 1)],
            vec![workspace(1, "DP-1", 0)],
            Some(1)
        ));

        assert!(bare.unfolds_further_than(&folded));
        assert!(!folded.unfolds_further_than(&bare));
        assert!(!bare.unfolds_further_than(&bare));
    }
}
