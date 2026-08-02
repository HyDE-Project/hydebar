//! The bridge from the privacy service to the event bus.
//!
//! The service pushes its events through a [`PrivacyEventPublisher`]; the
//! bridge here forwards each one as a [`PrivacyMessage`](super::PrivacyMessage)
//! and never fails, so the listener loop only stops when the service does.

use std::future::{Ready, ready};

use super::PrivacyMessage;
use crate::{
    ModuleEventSender,
    services::{
        ServiceEvent,
        privacy::{PrivacyEventPublisher, PrivacyService, State, error::PrivacyError}
    }
};

pub(super) struct ModulePublisher {
    sender: ModuleEventSender<PrivacyMessage>
}

impl ModulePublisher {
    pub(super) const fn new(sender: ModuleEventSender<PrivacyMessage>) -> Self {
        Self {
            sender
        }
    }
}

impl PrivacyEventPublisher for ModulePublisher {
    type SendFuture<'a>
        = Ready<Result<(), PrivacyError>>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<PrivacyService>) -> Self::SendFuture<'_> {
        self.sender.send(PrivacyMessage::Event(event));

        ready(Ok(()))
    }
}

pub(super) async fn run_start_listening<P>(
    state: State,
    publisher: &mut P
) -> Result<State, PrivacyError>
where
    P: PrivacyEventPublisher + Send
{
    PrivacyService::start_listening(state, publisher).await
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::{
        ModuleContext,
        event_bus::{BusEvent, EventBus, ModuleEvent}
    };

    #[tokio::test]
    async fn the_publisher_forwards_service_events_to_the_bus() {
        let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
        let mut receiver = bus.receiver();
        let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let mut publisher = ModulePublisher::new(ctx.module_sender(ModuleEvent::Privacy));
        publisher
            .send(ServiceEvent::Error(PrivacyError::WebcamUnavailable))
            .await
            .expect("publishing never fails");

        match receiver.try_recv() {
            Some(BusEvent::Module(ModuleEvent::Privacy(PrivacyMessage::Event(
                ServiceEvent::Error(PrivacyError::WebcamUnavailable)
            )))) => {}
            other => panic!("unexpected event: {other:?}")
        }
    }
}
