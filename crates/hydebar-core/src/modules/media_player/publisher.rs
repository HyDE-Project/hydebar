//! Bridge from the MPRIS listener to the module event bus.

use std::{
    future::{Future, ready},
    pin::Pin
};

use super::Message;
use crate::{
    ModuleEventSender,
    modules::ModuleError,
    services::{
        ServiceEvent,
        mpris::{MprisEventPublisher, MprisPlayerService}
    }
};

pub(super) struct MediaPlayerPublisher {
    sender: ModuleEventSender<Message>
}

impl MediaPlayerPublisher {
    pub(super) const fn new(sender: ModuleEventSender<Message>) -> Self {
        Self {
            sender
        }
    }
}

impl MprisEventPublisher for MediaPlayerPublisher {
    fn send(
        &mut self,
        event: ServiceEvent<MprisPlayerService>
    ) -> Pin<Box<dyn Future<Output = Result<(), ModuleError>> + Send + '_>> {
        self.sender.send(Message::Event(Box::new(event)));

        Box::pin(ready(Ok(())))
    }
}
