//! Generic spawner for service commands whose runner always yields an
//! event.

use log::warn;
use tokio::runtime::Handle;

use super::super::state::Message;
use crate::services::{ReadOnlyService, ServiceEvent};

pub(super) struct EventCommandParams<S, Command, Fut, Msg>
where
    S: Send + Clone + ReadOnlyService + 'static,
    Command: Send + 'static,
    Fut: std::future::Future<Output = ServiceEvent<S>> + Send + 'static,
    Msg: Send + 'static
{
    pub(super) runtime:      Option<Handle>,
    pub(super) sender:       Option<crate::ModuleEventSender<Message>>,
    pub(super) service:      Option<S>,
    pub(super) command:      Command,
    pub(super) runner:       fn(S, Command) -> Fut,
    pub(super) message_ctor: fn(Msg) -> Message,
    pub(super) event_ctor:   fn(ServiceEvent<S>) -> Msg,
    pub(super) service_name: &'static str
}

pub(super) fn spawn_event_command<S, Command, Fut, Msg>(
    params: EventCommandParams<S, Command, Fut, Msg>
) -> bool
where
    S: Send + Clone + ReadOnlyService + 'static,
    Command: Send + 'static,
    Fut: std::future::Future<Output = ServiceEvent<S>> + Send + 'static,
    Msg: Send + 'static
{
    if let (Some(handle), Some(sender), Some(service)) =
        (params.runtime, params.sender, params.service)
    {
        let runner = params.runner;
        let message_ctor = params.message_ctor;
        let event_ctor = params.event_ctor;
        let command = params.command;
        handle.spawn(async move {
            let event = runner(service, command).await;
            sender.send(message_ctor(event_ctor(event)));
        });
        true
    } else {
        warn!(
            "{} command ignored because runtime, sender, or service is unavailable",
            params.service_name
        );
        false
    }
}
