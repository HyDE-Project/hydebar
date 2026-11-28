//! The state a single custom module instance carries.
//!
//! What the producing command last said, what it last failed with, and the
//! record of the shell that feeds it. Starting and stopping that shell is
//! [`registration`](super::registration).

use std::{sync::Arc, time::Duration};

use hydebar_proto::config::CustomModuleSource;
use tokio::task::JoinHandle;

use super::{
    super::{data::CustomListenData, error::CustomCommandError},
    message::Message
};
use crate::{ModuleEventSender, services::ServiceEvent};

/// State of a single custom module instance.
#[derive(Default, Debug)]
pub struct Custom {
    pub(in crate::modules::custom_module) data: CustomListenData,
    pub(in crate::modules::custom_module) last_error: Option<CustomCommandError>,
    /// What the desktop's own menu for this module offers, glyph and name.
    ///
    /// Read once, where the module is made: a button module says nothing of
    /// itself, and this is the only thing it can show on a canvas that opens
    /// every module into a block of its own.
    choices: Vec<(String, String)>,
    pub(super) registration: Option<CustomRegistration>,
    pub(super) sender: Option<ModuleEventSender<Message>>,
    pub(super) listener_task: Option<JoinHandle<()>>
}

/// The shell feeding a module, named after the module it feeds.
#[derive(Debug, Clone)]
pub(super) struct CustomRegistration {
    pub(super) name:   Arc<str>,
    pub(super) source: RegistrationSource
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
    /// A module that carries the choices the desktop's menu for it offers.
    #[must_use]
    pub fn with_choices(choices: Vec<(String, String)>) -> Self {
        let mut module = Self::default();
        module.choices = choices;

        module
    }

    /// What the desktop's own menu for this module offers, glyph and name.
    #[must_use]
    pub fn choices(&self) -> &[(String, String)] {
        &self.choices
    }

    /// What the producing command last said.
    ///
    /// The same text the strip draws, and the tooltip beside it: a canvas
    /// naming a custom module says what the module says, not a second
    /// reading of its own.
    #[must_use]
    pub const fn readings(&self) -> &CustomListenData {
        &self.data
    }

    /// The last failure of the producing command, if it failed.
    #[must_use]
    pub const fn failure(&self) -> Option<&CustomCommandError> {
        self.last_error.as_ref()
    }

    /// Whether a command is currently feeding this module.
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

    /// Tears the feeding task down, leaving the registration standing.
    pub(super) fn abort_listener(&mut self) {
        if let Some(handle) = self.listener_task.take() {
            handle.abort();
        }
    }

    /// Folds one thing the command said into the state.
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
}

impl Drop for Custom {
    fn drop(&mut self) {
        self.abort_listener();
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::time::Duration;

    use hydebar_proto::config::CustomModuleSource;

    use super::{super::message::CustomCommandService, *};

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
