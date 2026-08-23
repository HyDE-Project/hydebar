//! Registration of the keyboard event stream the layout indicator owns.
//!
//! Background updates are delivered via the shared module event sender: the
//! listener publishes layout changes onto the event bus and the bar folds
//! them in through
//! [`KeyboardLayout::update`](super::KeyboardLayout::update).

use std::sync::Arc;

use super::{KeyboardLayout, listener};
use crate::{
    ModuleContext,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

impl<M> Module<M> for KeyboardLayout
where
    M: 'static
{
    type ViewData<'a> = ();
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::KeyboardLayout));

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(listener::run(hyprland, sender)));
        }

        Ok(())
    }

    /// Drops the keyboard event stream once the layout indicator leaves the
    /// bar.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }
}
