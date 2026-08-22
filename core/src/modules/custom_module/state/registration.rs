//! Starting and stopping the shell behind a custom module.

use std::{sync::Arc, time::Duration};

use hydebar_proto::config::CustomModuleSource;
use log::error;

use super::{
    super::{
        error::{CustomCommandError, CustomListenerError},
        listener::{run_custom_listener, send_event},
        poller::run_custom_poller
    },
    Custom, Message
};
use crate::{
    ModuleContext, ModuleEventSender, config::CustomModuleDef, event_bus::ModuleEvent,
    services::ServiceEvent
};

#[derive(Debug, Clone)]
pub(super) struct CustomRegistration {
    name:   Arc<str>,
    source: RegistrationSource
}

/// Producing command of a registered module, together with its schedule.
#[derive(Debug, Clone)]
pub(super) enum RegistrationSource {
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
    pub(super) fn from_config(source: CustomModuleSource<'_>) -> Self {
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
    /// Tears down the feeding task and forgets the registration.
    pub(in crate::modules::custom_module) fn stop_listener(&mut self) {
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
    pub(in crate::modules::custom_module) fn start_listener(
        &mut self,
        ctx: &ModuleContext,
        config: &CustomModuleDef
    ) {
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
