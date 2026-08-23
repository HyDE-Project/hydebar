//! Registration of the compositor stream the window title owns.
//!
//! No iced subscription is involved: the listener publishes focused-window
//! changes onto the event bus and the bar folds them in through
//! [`WindowTitle::update`](super::WindowTitle::update).

use std::sync::Arc;

use super::{WindowTitle, listener};
use crate::{
    ModuleContext,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

impl<M> Module<M> for WindowTitle
where
    M: 'static
{
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::WindowTitle));

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(listener::run(hyprland, sender)));
        }

        Ok(())
    }

    /// Drops the compositor event stream once the title leaves the bar.
    ///
    /// Focus changes are among the most frequent events a session produces, so
    /// a listener nobody renders is the most expensive one to leave running.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }
}
