//! Runtime state and listener wiring for a custom module.

use std::{sync::Arc, time::Duration};

use hydebar_proto::config::CustomModuleSource;
use iced::Subscription;
use log::error;
use tokio::task::JoinHandle;

use super::{
    data::CustomListenData,
    error::{CustomCommandError, CustomListenerError},
    listener::{run_custom_listener, send_event},
    poller::run_custom_poller
};
use crate::{
    ModuleContext, ModuleEventSender, config::CustomModuleDef, event_bus::ModuleEvent,
    services::ServiceEvent
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
    name:   Arc<str>,
    source: RegistrationSource
}

/// Producing command of a registered module, together with its schedule.
#[derive(Debug, Clone)]
enum RegistrationSource {
    /// One long lived process streaming json lines.
    Stream { command: Arc<str> },
    /// A command re-run on a schedule or when a real time signal arrives.
    Poll {
        command: Arc<str>,
        period:  Option<Duration>,
        signal:  Option<u8>
    }
}

impl RegistrationSource {
    fn from_config(source: CustomModuleSource<'_>) -> Self {
        match source {
            CustomModuleSource::Stream {
                command
            } => Self::Stream {
                command: Arc::from(command)
            },
            CustomModuleSource::Poll {
                command,
                interval,
                signal
            } => Self::Poll {
                command: Arc::from(command),
                period: interval.map(Duration::from_secs),
                signal
            }
        }
    }
}

impl Custom {
    /// Reports whether a producing command is currently feeding the module.
    ///
    /// Registration is what starts the shell behind a custom module, and
    /// the only externally visible trace of it is the task it left
    /// behind. The bar gates registration on the module being drawn
    /// somewhere, so this is the question a caller has to be able
    /// to ask to tell a module that is merely silent from one that
    /// was never started.
    #[must_use]
    pub const fn is_listening(&self) -> bool {
        self.registration.is_some()
    }

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

    /// Tears down the feeding task and forgets the registration.
    pub(super) fn stop_listener(&mut self) {
        self.abort_listener();
        self.sender = None;
        self.last_error = None;
        self.registration = None;
    }

    /// Restarts the task feeding the module from the given configuration.
    ///
    /// A definition without a schedule keeps the streaming listener, while
    /// an `interval` or a `signal` switches to the poller. Because
    /// the whole task is torn down first, a configuration reload
    /// that only changes the interval or the signal number restarts
    /// the module on the new schedule.
    pub(super) fn start_listener(&mut self, ctx: &ModuleContext, config: &CustomModuleDef) {
        self.stop_listener();
        self.registration = config.source().map(|source| CustomRegistration {
            name:   Arc::from(config.name.as_str()),
            source: RegistrationSource::from_config(source)
        });

        let Some(registration) = self.registration.clone() else {
            return;
        };

        let module_name_for_sender = Arc::clone(&registration.name);
        let sender = ctx.module_sender(move |message| ModuleEvent::Custom {
            name: Arc::clone(&module_name_for_sender),
            message
        });

        self.sender = Some(sender.clone());
        let module_name = Arc::clone(&registration.name);
        let source = registration.source;
        let error_sender = sender.clone();

        self.listener_task = Some(ctx.runtime_handle().spawn(async move {
            let outcome = match source {
                RegistrationSource::Stream {
                    command
                } => run_custom_listener(Arc::clone(&module_name), command, sender).await,
                RegistrationSource::Poll {
                    command,
                    period,
                    signal
                } => {
                    run_custom_poller(Arc::clone(&module_name), command, period, signal, sender)
                        .await
                }
            };

            report_listener_outcome(outcome, &module_name, &error_sender);
        }));
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

            if !matches!(error, CustomCommandError::ChannelClosed) {
                send_event(error_sender, ServiceEvent::Error(error));
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn update_payload(alt: &str) -> CustomListenData {
        CustomListenData {
            alt: String::from(alt),
            ..CustomListenData::default()
        }
    }

    #[test]
    fn an_update_replaces_the_data_and_clears_the_previous_error() {
        let mut module = Custom::default();
        module.update(Message::Event(ServiceEvent::Error(
            CustomCommandError::ChannelClosed
        )));

        module.update(Message::Event(ServiceEvent::Update(update_payload("42"))));

        assert_eq!(module.data.alt, "42");
        assert!(module.last_error.is_none());
    }

    #[test]
    fn an_error_keeps_the_data_already_on_screen() {
        let mut module = Custom::default();
        module.update(Message::Event(ServiceEvent::Update(update_payload("42"))));

        module.update(Message::Event(ServiceEvent::Error(
            CustomCommandError::ChannelClosed
        )));

        assert_eq!(module.data.alt, "42");
        assert!(matches!(
            module.last_error,
            Some(CustomCommandError::ChannelClosed)
        ));
    }

    #[test]
    fn an_init_event_changes_nothing() {
        let mut module = Custom::default();
        module.update(Message::Event(ServiceEvent::Update(update_payload("42"))));
        module.update(Message::Event(ServiceEvent::Error(
            CustomCommandError::ChannelClosed
        )));

        module.update(Message::Event(ServiceEvent::Init(CustomCommandService)));

        assert_eq!(module.data.alt, "42");
        assert!(matches!(
            module.last_error,
            Some(CustomCommandError::ChannelClosed)
        ));
    }

    #[test]
    fn a_module_without_a_registration_is_not_listening() {
        assert!(!Custom::default().is_listening());
    }

    #[test]
    fn a_stream_config_keeps_its_command_verbatim() {
        let source = RegistrationSource::from_config(CustomModuleSource::Stream {
            command: "tail -f log"
        });

        match source {
            RegistrationSource::Stream {
                command
            } => assert_eq!(command.as_ref(), "tail -f log"),
            other @ RegistrationSource::Poll {
                ..
            } => panic!("unexpected source: {other:?}")
        }
    }

    #[test]
    fn a_poll_config_turns_interval_seconds_into_a_period() {
        let source = RegistrationSource::from_config(CustomModuleSource::Poll {
            command:  "hyde-shell cpuinfo",
            interval: Some(5),
            signal:   Some(20)
        });

        match source {
            RegistrationSource::Poll {
                command,
                period,
                signal
            } => {
                assert_eq!(command.as_ref(), "hyde-shell cpuinfo");
                assert_eq!(period, Some(Duration::from_secs(5)));
                assert_eq!(signal, Some(20));
            }
            other @ RegistrationSource::Stream {
                ..
            } => panic!("unexpected source: {other:?}")
        }
    }

    #[test]
    fn a_poll_config_without_an_interval_has_no_period() {
        let source = RegistrationSource::from_config(CustomModuleSource::Poll {
            command:  "checkupdates",
            interval: None,
            signal:   None
        });

        match source {
            RegistrationSource::Poll {
                period,
                signal,
                ..
            } => {
                assert!(period.is_none());
                assert!(signal.is_none());
            }
            other @ RegistrationSource::Stream {
                ..
            } => panic!("unexpected source: {other:?}")
        }
    }
}
