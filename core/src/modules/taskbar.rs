//! Bar strip of the mapped windows, pressed to focus one.
//!
//! The counterpart of the reference bar's taskbar: one entry per mapped
//! client, drawn with the icon its class resolves to in the icon theme, or
//! the first letter of the class where no icon answers. The focused window
//! is drawn at full strength and the rest step back, so the strip reads as
//! "where am I" as much as "what is open".
//!
//! The list is a snapshot re-read on every window event, the same rhythm the
//! workspaces module keeps: the compositor pushes, the bar asks once per
//! burst, and an unchanged answer is dropped before it reaches the bus.
//!
//! One folder, four rooms: [`state`] folds messages in and dispatches focus
//! commands, [`listener`] follows the compositor's window events in the
//! background, [`view`] draws the strip and [`module`] wires the module to
//! the bar. The root holds the state the rooms share.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandClientInfo, HyprlandPort};
use tokio::{runtime::Handle, task::JoinHandle};

use crate::ModuleEventSender;

mod listener;
mod module;
mod state;
mod view;

/// Messages delivered to the taskbar.
#[derive(Debug, Clone)]
pub enum Message {
    /// A fresh client list arrived from the compositor.
    ClientsChanged(Vec<HyprlandClientInfo>),
    /// An entry was pressed and its window wants the focus.
    Focus(String)
}

/// Bar strip of the compositor's mapped clients.
pub struct Taskbar {
    hyprland: Arc<dyn HyprlandPort>,
    clients:  Vec<HyprlandClientInfo>,
    sender:   Option<ModuleEventSender<Message>>,
    runtime:  Option<Handle>,
    task:     Option<JoinHandle<()>>
}

impl std::fmt::Debug for Taskbar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Taskbar")
            .field("hyprland", &"<HyprlandPort>")
            .field("clients", &self.clients)
            .field("sender", &self.sender)
            .field("runtime", &self.runtime)
            .field("task", &self.task)
            .finish()
    }
}

impl Taskbar {
    #[must_use]
    pub fn new(hyprland: Arc<dyn HyprlandPort>) -> Self {
        Self {
            hyprland,
            clients: Vec::new(),
            sender: None,
            runtime: None,
            task: None
        }
    }
}

#[cfg(test)]
pub(crate) fn test_client(address: &str, focused: bool) -> HyprlandClientInfo {
    HyprlandClientInfo {
        address: address.to_owned(),
        class: "kitty".to_owned(),
        title: "shell".to_owned(),
        workspace_id: 1,
        focused,
        floating: false
    }
}
