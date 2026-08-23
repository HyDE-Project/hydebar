//! Registration of the compositor stream the workspaces indicators own.
//!
//! Background updates are delivered via the shared module event sender: the
//! listener publishes snapshots onto the event bus and the bar folds them in
//! through [`Workspaces::update`](super::Workspaces::update).

use std::sync::Arc;

use super::{Workspaces, listener};
use crate::{
    ModuleContext,
    config::WorkspacesModuleConfig,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

impl<M> Module<M> for Workspaces
where
    M: 'static
{
    type RegistrationData<'a> = &'a WorkspacesModuleConfig;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::Workspaces));
        self.runtime = Some(ctx.runtime_handle().clone());

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(listener::run(hyprland, sender)));
        }

        Ok(())
    }

    /// Drops the compositor event stream once the module leaves the bar.
    ///
    /// The listener holds an open Hyprland socket and republishes on every
    /// window and workspace change; a layout without workspaces would repaint
    /// the bar for each of them and show nothing new.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }
}
