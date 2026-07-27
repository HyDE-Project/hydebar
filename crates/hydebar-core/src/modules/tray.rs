use std::{future::Future, pin::Pin, sync::Arc};

use tokio::{runtime::Handle, task::JoinHandle};

use crate::{
    ModuleEventSender,
    services::{
        ServiceEvent,
        tray::{TrayCommand, TrayService}
    }
};

#[derive(Debug, Clone)]
pub enum TrayMessage {
    Event(Box<ServiceEvent<TrayService>>),
    ToggleSubmenu(i32),
    MenuSelected(String, i32)
}

type ListenerSpawner =
    Arc<dyn Fn(ModuleEventSender<TrayMessage>, Handle) -> JoinHandle<()> + Send + Sync>;
type CommandFactory =
    Arc<dyn Fn(Option<&TrayService>, TrayCommand) -> Option<TrayCommandFuture> + Send + Sync>;
type TrayCommandFuture = Pin<Box<dyn Future<Output = ServiceEvent<TrayService>> + Send + 'static>>;

pub struct TrayModule {
    pub service:      Option<TrayService>,
    pub submenus:     Vec<i32>,
    sender:           Option<ModuleEventSender<TrayMessage>>,
    runtime:          Option<Handle>,
    listener_handles: Vec<JoinHandle<()>>,
    listener_spawner: ListenerSpawner,
    command_factory:  CommandFactory
}

impl std::fmt::Debug for TrayModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayModule")
            .field("service", &self.service)
            .field("submenus", &self.submenus)
            .field("sender", &self.sender)
            .field("runtime", &self.runtime)
            .field(
                "listener_handles",
                &format!("<{} handles>", self.listener_handles.len())
            )
            .field("listener_spawner", &"<function>")
            .field("command_factory", &"<function>")
            .finish()
    }
}

mod module;
mod state;
mod view;

#[cfg(test)]
mod tests;
