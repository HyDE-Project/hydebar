use std::{
    future::{Future, ready},
    pin::Pin
};

use tokio::{runtime::Handle, task::JoinHandle};

use super::ModuleError;
use crate::{
    ModuleEventSender,
    services::{
        ServiceEvent,
        mpris::{MprisEventPublisher, MprisPlayerService}
    }
};

#[derive(Default)]
pub struct MediaPlayer {
    service: Option<MprisPlayerService>,
    sender:  Option<ModuleEventSender<Message>>,
    runtime: Option<Handle>,
    tasks:   Vec<JoinHandle<()>>
}

struct MediaPlayerPublisher {
    sender: ModuleEventSender<Message>
}

impl MediaPlayerPublisher {
    fn new(sender: ModuleEventSender<Message>) -> Self {
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
        Box::pin(ready(
            self.sender
                .try_send(Message::Event(event))
                .map_err(ModuleError::from)
        ))
    }
}

mod commands;
mod messages;
mod module;
mod state;
mod view;

#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests;

pub use messages::Message;
