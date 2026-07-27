//! Runtime state and listener wiring for a custom module.

use std::sync::Arc;

use iced::Subscription;
use log::error;
use tokio::task::JoinHandle;

use super::{
    data::CustomListenData,
    error::{CustomCommandError, CustomListenerError},
    listener::{run_custom_listener, send_event}
};
use crate::{
    ModuleContext, ModuleEventSender, config::CustomModuleDef, event_bus::ModuleEvent,
    modules::ModuleError, services::ServiceEvent
};

/// State of a single custom module instance.
#[derive(Default, Debug)]
pub struct Custom {
    pub(super) data:       CustomListenData,
    pub(super) last_error: Option<CustomCommandError>,
    registration:          Option<CustomRegistration>,
    sender:                Option<ModuleEventSender<Message>>,
    listener_task:         Option<JoinHandle<()>>
}

#[derive(Debug, Clone)]
struct CustomRegistration {
    name:           Arc<str>,
    listen_command: Arc<str>
}

impl Custom {
    fn abort_listener(&mut self) {
        if let Some(handle) = self.listener_task.take() {
            handle.abort();
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Event(ServiceEvent::Update(data)) => {
                self.data = data;
                self.last_error = None;
            }
            Message::Event(ServiceEvent::Error(error)) => {
                self.last_error = Some(error);
            }
            Message::Event(ServiceEvent::Init(_)) => {}
        }
    }

    /// Restarts the listener process described by the given configuration.
    pub(super) fn start_listener(
        &mut self,
        ctx: &ModuleContext,
        config: Option<&CustomModuleDef>
    ) -> Result<(), ModuleError> {
        self.abort_listener();
        self.sender = None;
        self.last_error = None;
        self.registration = config.and_then(|definition| {
            definition
                .listen_cmd
                .as_ref()
                .map(|command| CustomRegistration {
                    name:           Arc::from(definition.name.as_str()),
                    listen_command: Arc::from(command.as_str())
                })
        });

        let Some(registration) = self.registration.clone() else {
            return Ok(());
        };

        let module_name_for_sender = Arc::clone(&registration.name);
        let sender = ctx.module_sender(move |message| ModuleEvent::Custom {
            name: Arc::clone(&module_name_for_sender),
            message
        });

        self.sender = Some(sender.clone());
        let module_name = Arc::clone(&registration.name);
        let listen_command = Arc::clone(&registration.listen_command);
        let error_sender = sender.clone();

        self.listener_task = Some(ctx.runtime_handle().spawn(async move {
            report_listener_outcome(
                run_custom_listener(module_name.clone(), listen_command, sender).await,
                &module_name,
                &error_sender
            );
        }));

        Ok(())
    }
}

fn report_listener_outcome(
    outcome: Result<(), CustomListenerError>,
    module_name: &Arc<str>,
    error_sender: &ModuleEventSender<Message>
) {
    match outcome {
        Ok(()) => {}
        Err(CustomListenerError::Command(error)) => {
            error!("Custom module '{module_name}' listener terminated with error: {error:?}");

            if !matches!(error, CustomCommandError::ChannelClosed)
                && let Err(send_error) = send_event(error_sender, ServiceEvent::Error(error))
            {
                error!(
                    "Custom module '{module_name}' failed to publish error notification: \
                     {send_error}"
                );
            }
        }
        Err(CustomListenerError::Module(error)) => {
            error!("Custom module '{module_name}' failed to publish event: {error}");
        }
    }
}

impl Drop for Custom {
    fn drop(&mut self) {
        self.abort_listener();
    }
}

/// Messages delivered to a custom module.
#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<CustomCommandService>)
}

/// Marker service carrying listener updates through the event bus.
#[derive(Debug, Clone, Default)]
pub struct CustomCommandService;

impl crate::services::ReadOnlyService for CustomCommandService {
    type UpdateEvent = CustomListenData;
    type Error = CustomCommandError;

    fn update(&mut self, _event: Self::UpdateEvent) {}

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::none()
    }
}
