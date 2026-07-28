//! Module rendering implementation for App - GUI layer only

pub(super) mod actions;
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
    /// Action bound to the right mouse button.
    ///
    /// It carries an [`OnModulePress`] rather than a bare message because a
    /// custom module declaring a context menu opens it from here, and a menu
    /// has to be anchored under the module it belongs to.
    right:  Option<OnModulePress<Message>>,
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
