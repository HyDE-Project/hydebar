//! The keyboard layout indicator: the active layout's label in the bar.
//!
//! One folder, three rooms: [`state`] folds messages in and steps the
//! label's dissolve, [`listener`] follows the compositor's keyboard events
//! in the background and [`module`] wires the module to the bar. The root
//! holds the state the rooms share.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandKeyboardState, HyprlandPort};
use tokio::task::JoinHandle;

use crate::ModuleEventSender;

mod listener;
mod module;
mod state;

/// Bar entry naming the keyboard layout in force.
pub struct KeyboardLayout {
    hyprland:        Arc<dyn HyprlandPort>,
    multiple_layout: bool,
    active:          String,
    sender:          Option<ModuleEventSender<Message>>,
    task:            Option<JoinHandle<()>>,
    shown:           crate::components::crossfade::Crossfade
}

impl std::fmt::Debug for KeyboardLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardLayout")
            .field("hyprland", &"<HyprlandPort>")
            .field("multiple_layout", &self.multiple_layout)
            .field("active", &self.active)
            .field("sender", &self.sender)
            .field("task", &self.task.as_ref().map(|_| "<JoinHandle>"))
            .field("shown", &self.shown)
            .finish()
    }
}

impl Clone for KeyboardLayout {
    /// The running listener task stays behind: a [`JoinHandle`] cannot be
    /// cloned, so the clone starts without one.
    fn clone(&self) -> Self {
        Self {
            hyprland:        Arc::clone(&self.hyprland),
            multiple_layout: self.multiple_layout,
            active:          self.active.clone(),
            sender:          self.sender.clone(),
            task:            None,
            shown:           self.shown.clone()
        }
    }
}

/// What the keyboard layout entry answers to.
#[derive(Debug, Clone)]
pub enum Message {
    /// Whether there is more than one layout to step to.
    LayoutConfigChanged(bool),
    /// The layout in force changed.
    ActiveLayoutChanged(String),
    /// Step to the next layout.
    ChangeLayout
}

impl KeyboardLayout {
    /// An entry that has not read the keyboard yet.
    pub fn new(hyprland: Arc<dyn HyprlandPort>) -> Self {
        let HyprlandKeyboardState {
            active_layout,
            has_multiple_layouts,
            ..
        } = hyprland
            .keyboard_state()
            .unwrap_or_else(|_| HyprlandKeyboardState {
                active_layout:        "unknown".to_string(),
                has_multiple_layouts: false,
                active_submap:        None
            });

        Self {
            hyprland,
            multiple_layout: has_multiple_layouts,
            active: active_layout,
            sender: None,
            task: None,
            shown: crate::components::crossfade::Crossfade::default()
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_from_keyboard_state() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port;

        let module = KeyboardLayout::new(port_trait);

        assert_eq!(module.active_layout(), "us");
        assert!(module.has_multiple_layouts());
    }
}
