//! What a custom module answers to, and the service that carries it.

use iced::Subscription;

use super::super::{data::CustomListenData, error::CustomCommandError};
use crate::services::ServiceEvent;

/// Messages delivered to a custom module.
#[derive(Debug, Clone)]
pub enum Message {
    /// The command said something.
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
