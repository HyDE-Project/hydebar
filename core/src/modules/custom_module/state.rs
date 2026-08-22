//! Runtime state and listener wiring for a custom module.
//!
//! Two rooms. The state itself is here — what the module knows, what it was
//! told last and whether anything is feeding it — and [`registration`] is the
//! starting and stopping of the shell behind it.

mod registration;

use iced::Subscription;
use tokio::task::JoinHandle;

use self::registration::CustomRegistration;
#[cfg(test)]
use self::registration::RegistrationSource;
use super::{data::CustomListenData, error::CustomCommandError};
use crate::{ModuleEventSender, services::ServiceEvent};

/// State of a single custom module instance.
#[derive(Default, Debug)]
pub struct Custom {
    pub(super) data:       CustomListenData,
    pub(super) last_error: Option<CustomCommandError>,
    registration:          Option<CustomRegistration>,
    sender:                Option<ModuleEventSender<Message>>,
    listener_task:         Option<JoinHandle<()>>
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::time::Duration;

    use hydebar_proto::config::CustomModuleSource;

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
