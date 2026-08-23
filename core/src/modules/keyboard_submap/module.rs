//! Registration of the compositor stream the submap indicator owns.

use std::sync::Arc;

use super::{KeyboardSubmap, listener};
use crate::{
    ModuleContext,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

impl<M> Module<M> for KeyboardSubmap
where
    M: 'static
{
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::KeyboardSubmap));

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(listener::run(hyprland, sender)));
        }

        Ok(())
    }

    /// Drops the submap event stream once the indicator leaves the bar.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }
}
