//! Bridges from the service event streams onto the module message bus.
//!
//! One generic forwarder instead of one hand-written struct per service:
//! the five bridges differed only in the message constructor and the
//! name used in the failure log, which is exactly what a value can
//! carry.

use std::future::{Ready, ready};

use log::warn;

use super::{
    audio::AudioMessage, bluetooth::BluetoothMessage, brightness::BrightnessMessage,
    network::NetworkMessage, state::Message, upower::UPowerMessage
};
use crate::{
    ModuleEventSender,
    services::{
        ReadOnlyService, ServiceEvent, ServiceEventPublisher, audio::AudioService,
        bluetooth::BluetoothService, brightness::BrightnessService, network::NetworkService,
        upower::UPowerService
    }
};

/// Forwards the events of one service, wrapped into the module's message.
pub(super) struct EventForwarder<S: ReadOnlyService> {
    sender: ModuleEventSender<Message>,
    wrap:   fn(ServiceEvent<S>) -> Message,
    label:  &'static str
}

impl<S: ReadOnlyService> EventForwarder<S> {
    fn new(
        sender: ModuleEventSender<Message>,
        wrap: fn(ServiceEvent<S>) -> Message,
        label: &'static str
    ) -> Self {
        Self {
            sender,
            wrap,
            label
        }
    }
}

impl<S: ReadOnlyService> ServiceEventPublisher<S> for EventForwarder<S> {
    type SendFuture<'a>
        = Ready<()>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<S>) -> Self::SendFuture<'_> {
        if let Err(err) = self.sender.try_send((self.wrap)(event)) {
            warn!("failed to publish {} event: {err}", self.label);
        }

        ready(())
    }
}

/// The forwarder of the audio service.
pub(super) fn audio_forwarder(
    sender: ModuleEventSender<Message>
) -> EventForwarder<AudioService> {
    EventForwarder::new(
        sender,
        |event| Message::Audio(AudioMessage::Event(Box::new(event))),
        "audio"
    )
}

/// The forwarder of the brightness service.
pub(super) fn brightness_forwarder(
    sender: ModuleEventSender<Message>
) -> EventForwarder<BrightnessService> {
    EventForwarder::new(
        sender,
        |event| Message::Brightness(BrightnessMessage::Event(Box::new(event))),
        "brightness"
    )
}

/// The forwarder of the network service.
pub(super) fn network_forwarder(
    sender: ModuleEventSender<Message>
) -> EventForwarder<NetworkService> {
    EventForwarder::new(
        sender,
        |event| Message::Network(NetworkMessage::Event(Box::new(event))),
        "network"
    )
}

/// The forwarder of the bluetooth service.
pub(super) fn bluetooth_forwarder(
    sender: ModuleEventSender<Message>
) -> EventForwarder<BluetoothService> {
    EventForwarder::new(
        sender,
        |event| Message::Bluetooth(BluetoothMessage::Event(Box::new(event))),
        "bluetooth"
    )
}

/// The forwarder of the power service.
pub(super) fn upower_forwarder(
    sender: ModuleEventSender<Message>
) -> EventForwarder<UPowerService> {
    EventForwarder::new(
        sender,
        |event| Message::UPower(UPowerMessage::Event(Box::new(event))),
        "upower"
    )
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use tokio::runtime::Runtime;

    use super::*;
    use crate::{
        ModuleContext, ModuleEventSender,
        event_bus::{BusEvent, EventBus, EventReceiver, ModuleEvent},
        modules::control_center::Message
    };

    fn setup_forwarder() -> (Runtime, EventReceiver, ModuleEventSender<Message>) {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let sender = bus.sender();
        let receiver = bus.receiver();
        let ctx = ModuleContext::new(sender, runtime.handle().clone());
        let module_sender = ctx.module_sender(ModuleEvent::ControlCenter);
        (runtime, receiver, module_sender)
    }

    #[test]
    fn audio_forwarder_enqueues_events() {
        let (runtime, mut receiver, sender) = setup_forwarder();
        let mut forwarder = audio_forwarder(sender);

        let _ = forwarder.send(ServiceEvent::Error(()));

        let event = receiver.try_recv().expect("event queued");
        match event {
            Some(BusEvent::Module(ModuleEvent::ControlCenter(Message::Audio(
                AudioMessage::Event(event)
            )))) if matches!(*event, ServiceEvent::Error(())) => {}
            other => panic!("unexpected event: {other:?}")
        }

        drop(runtime);
    }

    #[test]
    fn network_forwarder_enqueues_events() {
        let (runtime, mut receiver, sender) = setup_forwarder();
        let mut forwarder = network_forwarder(sender);

        let error = crate::services::network::NetworkServiceError::new("failure");
        let _ = forwarder.send(ServiceEvent::Error(error.clone()));

        let event = receiver.try_recv().expect("event queued");
        match event {
            Some(BusEvent::Module(ModuleEvent::ControlCenter(Message::Network(
                NetworkMessage::Event(received)
            )))) => {
                let ServiceEvent::Error(received) = *received else {
                    panic!("expected an error event");
                };
                assert_eq!(received.message(), error.message());
            }
            other => panic!("unexpected event: {other:?}")
        }

        drop(runtime);
    }
}
