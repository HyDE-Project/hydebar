//! Registration of the status notifier listener the tray owns.
//!
//! The tray draws no bar entry of its own: each icon of the strip toggles a
//! positioned menu, and those messages carry a
//! [`ButtonUIRef`](crate::position_button::ButtonUIRef) that no drawing
//! generic over its message type can construct, so the strip stays in the
//! bar layer that owns the surface.

use std::sync::Arc;

use super::super::{
    Module, ModuleError,
    tray::{CommandFactory, ListenerSpawner, TrayMessage, TrayModule}
};
use crate::{ModuleContext, event_bus::ModuleEvent, services::tray::TrayService};

impl<M> Module<M> for TrayModule
where
    M: 'static
{
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.abort_listener_handles();
        self.sender = Some(ctx.module_sender(ModuleEvent::Tray));
        self.runtime = Some(ctx.runtime_handle().clone());
        self.spawn_listener();

        Ok(())
    }

    /// Drops the status notifier listener once the tray leaves the bar.
    ///
    /// The listener owns the `StatusNotifierWatcher` name on D-Bus and
    /// receives every icon and menu change every tray application
    /// publishes. A bar without a tray area renders none of it, and
    /// holding the well known name would also keep other trays from
    /// taking over.
    fn deregister(&mut self) {
        self.abort_listener_handles();

        self.service = None;
        self.sender = None;
    }
}

impl Default for TrayModule {
    fn default() -> Self {
        Self {
            service:          None,
            submenus:         Vec::new(),
            sender:           None,
            runtime:          None,
            listener_handles: Vec::new(),
            listener_spawner: default_listener_spawner(),
            command_factory:  default_command_factory()
        }
    }
}

impl Drop for TrayModule {
    fn drop(&mut self) {
        self.abort_listener_handles();
    }
}

pub(super) fn default_listener_spawner() -> ListenerSpawner {
    Arc::new(|sender, runtime| {
        runtime.spawn(async move {
            TrayService::start_listening(|event| {
                let sender = sender.clone();
                async move {
                    sender.send(TrayMessage::Event(Box::new(event)));
                }
            })
            .await;
        })
    })
}

pub(super) fn default_command_factory() -> CommandFactory {
    Arc::new(|service, command| service.and_then(|svc| svc.prepare_command(command)))
}
