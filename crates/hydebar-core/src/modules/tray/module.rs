//! Module trait wiring for the tray.

use std::sync::Arc;

use iced::{Element, window::Id};
use log::error;

use super::super::{
    Module, ModuleError, OnModulePress,
    tray::{CommandFactory, ListenerSpawner, TrayMessage, TrayModule}
};
use crate::{ModuleContext, event_bus::ModuleEvent, services::tray::TrayService};

impl<M> Module<M> for TrayModule
where
    M: 'static + Clone
{
    type ViewData<'a> = (Id, f32);
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.abort_listener_handles();
        self.sender = Some(ctx.module_sender(ModuleEvent::Tray));
        self.runtime = Some(ctx.runtime_handle().clone());
        self.spawn_listener();

        Ok(())
    }

    fn view(
        &self,
        (_id, _opacity): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        // TODO: Tray view needs special handling for position_button messages
        // This requires GUI layer integration as buttons need to construct messages
        // with ButtonUIRef which can't be done generically in core.
        // For now, disabled to allow compilation.
        None
    }

    fn subscription(&self) -> Option<iced::Subscription<M>> {
        None
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
                    if let Err(err) = sender.try_send(TrayMessage::Event(Box::new(event))) {
                        error!("failed to publish tray service event: {err}");
                    }
                }
            })
            .await;
        })
    })
}

pub(super) fn default_command_factory() -> CommandFactory {
    Arc::new(|service, command| service.and_then(|svc| svc.prepare_command(command)))
}
