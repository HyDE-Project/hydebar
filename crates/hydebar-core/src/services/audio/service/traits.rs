//! Service trait implementations for the audio service.

use std::{
    any::TypeId,
    ops::{Deref, DerefMut}
};

use iced::{Subscription, Task, stream::channel};

use super::super::{
    AudioCommand, AudioService,
    model::{AudioData, AudioEvent}
};
use crate::services::{ReadOnlyService, Service, ServiceEvent};

impl Deref for AudioService {
    type Target = AudioData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for AudioService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl ReadOnlyService for AudioService {
    type UpdateEvent = AudioEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.update_from_event(event);
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with(id, |&_id| {
            channel(100, |mut output| async move {
                AudioService::listen(&mut output).await;
            })
        })
    }
}

impl Service for AudioService {
    type Command = AudioCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        self.apply_command(command);
        Task::none()
    }
}
