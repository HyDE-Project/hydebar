//! The submap indicator: the compositor mode the keyboard is in.
//!
//! The compositor is in no submap most of the time, and the entry draws
//! nothing at all then; it appears the moment a binding puts the keyboard
//! into a mode, so the strip says what the keys mean right now.
//!
//! One folder, four rooms: [`state`] folds messages in and steps the label's
//! dissolve, [`listener`] follows the compositor's submap events in the
//! background, [`view`] paints the bar entry and [`module`] starts and stops
//! the listener with the layout. The root holds the state the rooms share.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandKeyboardState, HyprlandPort};
use tokio::task::JoinHandle;

use crate::ModuleEventSender;

mod listener;
mod module;
mod state;
mod view;

/// Bar entry naming the compositor submap the keyboard is in.
pub struct KeyboardSubmap {
    hyprland: Arc<dyn HyprlandPort>,
    submap:   String,
    sender:   Option<ModuleEventSender<Message>>,
    task:     Option<JoinHandle<()>>,
    shown:    crate::components::crossfade::Crossfade
}

impl std::fmt::Debug for KeyboardSubmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardSubmap")
            .field("hyprland", &"<HyprlandPort>")
            .field("submap", &self.submap)
            .field("sender", &self.sender)
            .field("task", &self.task)
            .field("shown", &self.shown)
            .finish()
    }
}

impl KeyboardSubmap {
    /// The submap the compositor is in, empty while it is in none.
    #[must_use]
    pub fn active(&self) -> &str {
        &self.submap
    }

    /// An entry that has not read the keyboard yet.
    pub fn new(hyprland: Arc<dyn HyprlandPort>) -> Self {
        let initial_submap = hyprland
            .keyboard_state()
            .unwrap_or(HyprlandKeyboardState {
                active_layout:        String::new(),
                has_multiple_layouts: false,
                active_submap:        None
            })
            .active_submap
            .unwrap_or_default();

        Self {
            hyprland,
            submap: initial_submap,
            sender: None,
            task: None,
            shown: crate::components::crossfade::Crossfade::default()
        }
    }
}

/// What the submap entry answers to.
#[derive(Debug, Clone)]
pub enum Message {
    /// The submap in force changed; empty means none.
    SubmapChanged(String)
}
