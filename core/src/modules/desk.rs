//! The desk: what the bar unfolds into when the screen is bare.
//!
//! The strip along the edge answers the questions a window leaves room for.
//! A workspace with nothing on it leaves room for all of them, and that is
//! what the desk is: the readouts that never fit the bar — the machine, the
//! link, the mounts, the hour at the size a room away can read it — drawn
//! straight onto the wallpaper for as long as no window wants the screen.
//!
//! One folder, four rooms: [`bareness`] reads the screens off a compositor
//! snapshot, [`listener`] follows the compositor's workspace events in the
//! background, [`state`] folds the answers in and [`module`] wires the desk to
//! the bar. The root holds the state the rooms share.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::HyprlandPort;
use tokio::task::JoinHandle;

use crate::ModuleEventSender;

mod bareness;
mod listener;
mod module;
mod state;

pub use bareness::Bareness;

/// Messages delivered to the desk.
#[derive(Debug, Clone)]
pub enum Message {
    /// A fresh answer to which screens hold no window.
    ScreensChanged(Bareness)
}

/// The canvas the bar unfolds into, and the screens it may unfold on.
pub struct Desk {
    hyprland: Arc<dyn HyprlandPort>,
    bareness: Bareness,
    sender:   Option<ModuleEventSender<Message>>,
    task:     Option<JoinHandle<()>>
}

impl Desk {
    #[must_use]
    pub fn new(hyprland: Arc<dyn HyprlandPort>) -> Self {
        Self {
            hyprland,
            bareness: Bareness::default(),
            sender: None,
            task: None
        }
    }
}

impl std::fmt::Debug for Desk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Desk")
            .field("hyprland", &"<HyprlandPort>")
            .field("bareness", &self.bareness)
            .field("sender", &self.sender)
            .field("task", &self.task)
            .finish()
    }
}

#[cfg(test)]
pub(crate) fn test_bareness() -> Bareness {
    use hydebar_proto::ports::hyprland::{
        HyprlandMonitorInfo, HyprlandWorkspaceInfo, HyprlandWorkspaceSnapshot
    };

    bareness::read(&HyprlandWorkspaceSnapshot {
        monitors:            vec![HyprlandMonitorInfo {
            id:                   0,
            name:                 "DP-1".to_owned(),
            active_workspace_id:  Some(1),
            special_workspace_id: None
        }],
        workspaces:          vec![HyprlandWorkspaceInfo {
            id:           1,
            name:         "1".to_owned(),
            monitor_id:   Some(0),
            monitor_name: "DP-1".to_owned(),
            window_count: 0
        }],
        active_workspace_id: Some(1)
    })
}
