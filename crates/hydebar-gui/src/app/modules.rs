//! Module rendering implementation for App - GUI layer only

mod actions;
mod dispatch;
mod element;
mod section;

use hydebar_core::modules::OnModulePress;

use super::state::Message;

/// Press actions a bar module reacts to, one per mouse button.
#[derive(Debug, Default)]
pub struct ModuleActions {
    /// Action bound to the left mouse button.
    left:   Option<OnModulePress<Message>>,
    /// Message published on a right press.
    right:  Option<Message>,
    /// Message published on a middle press.
    middle: Option<Message>
}

impl ModuleActions {
    /// Reports whether the module reacts to no mouse button at all.
    ///
    /// Inert modules render as plain indicators instead of unresponsive
    /// buttons.
    fn is_inert(&self) -> bool {
        self.left.is_none() && self.right.is_none() && self.middle.is_none()
    }
}
