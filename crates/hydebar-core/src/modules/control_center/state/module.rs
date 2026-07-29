//! Module trait wiring for the settings menu.

use std::time::Duration;

use log::warn;

use super::super::{
    ControlCenter, Message,
    event_forwarders::{
        AudioEventForwarder, BluetoothEventForwarder, BrightnessEventForwarder,
        NetworkEventForwarder, UPowerEventForwarder
    },
    network::NetworkMessage,
    view::ControlCenterViewExt
};
use crate::{
    ModuleContext,
    attention::PollSchedule,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError, OnModulePress},
    services::{
        ServiceEvent,
        audio::AudioService,
        bluetooth::BluetoothService,
        brightness::BrightnessService,
        network::{NetworkEvent, NetworkService},
        upower::UPowerService
    }
};

/// How often the attended network menu re-reads the radios in earshot.
const NEARBY_NETWORKS_INTERVAL: Duration = Duration::from_secs(2);

impl ControlCenter {
    /// Re-reads the nearby networks the open menu draws.
    ///
    /// A list identical to the one already on screen is dropped rather than
    /// published: every event reaching the bar rebuilds every surface it owns,
    /// and a band nobody is roaming reports the same radios sample after
    /// sample.
    pub(crate) fn refresh_access_points(&mut self, ctx: &ModuleContext) {
        if self
            .network_poll
            .as_ref()
            .is_some_and(|poll| !poll.is_finished())
        {
            return;
        }

        let (Some(service), Some(sender)) = (self.network.as_ref(), self.sender.clone()) else {
            return;
        };

        let service = service.clone();
        let known = service.wireless_access_points.clone();

        self.network_poll = Some(ctx.runtime_handle().spawn(async move {
            match service.access_points().await {
                Ok(access_points) => {
                    if access_points == known {
                        return;
                    }

                    if let Err(err) = sender.try_send(Message::Network(NetworkMessage::Event(
                        ServiceEvent::Update(NetworkEvent::WirelessAccessPoint(access_points))
                    ))) {
                        warn!("failed to publish the nearby networks: {err}");
                    }
                }
                Err(err) => warn!("failed to read the nearby networks: {err}")
            }
        }));
    }
}

impl<M> Module<M> for ControlCenter
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = <Self as ControlCenterViewExt>::ViewData<'a>;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        for task in self.tasks.drain(..) {
            task.abort();
        }

        let sender = ctx.module_sender(ModuleEvent::ControlCenter);

        let mut tasks = Vec::new();

        let mut audio_publisher = AudioEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            AudioService::listen(&mut audio_publisher).await;
        }));

        let mut brightness_publisher = BrightnessEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            BrightnessService::listen(&mut brightness_publisher).await;
        }));

        let mut network_publisher = NetworkEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            NetworkService::listen(&mut network_publisher).await;
        }));

        let mut bluetooth_publisher = BluetoothEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            BluetoothService::listen(&mut bluetooth_publisher).await;
        }));

        let mut upower_publisher = UPowerEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            UPowerService::listen(&mut upower_publisher).await;
        }));

        self.sender = Some(sender);
        self.runtime = Some(ctx.runtime_handle().clone());
        self.tasks = tasks;

        Ok(())
    }

    /// Disconnects the five hardware services once nothing on the bar shows
    /// them.
    ///
    /// Audio, brightness, network, bluetooth and UPower each hold a D-Bus or
    /// PulseAudio connection that reports on every volume step, every signal
    /// strength sample and every battery reading. Together they are the single
    /// largest idle cost the bar can carry, and a layout without any of their
    /// readouts pays it for nothing.
    ///
    /// The idle inhibitor is untouched: it belongs to the compositor session
    /// rather than to a service, and the module keeps rendering its state.
    fn deregister(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }

        if let Some(poll) = self.network_poll.take() {
            poll.abort();
        }

        self.sender = None;
    }

    /// Refreshes the nearby networks only while the menu showing them is
    /// attended.
    ///
    /// Nothing on the bar draws the list, so a resting cadence would pay a bus
    /// round trip per neighbouring radio for a readout behind a closed menu.
    fn poll_schedule(&self) -> Option<PollSchedule> {
        self.network
            .is_some()
            .then(|| PollSchedule::only_when_attended(NEARBY_NETWORKS_INTERVAL))
    }

    fn poll(&mut self, ctx: &ModuleContext) -> Result<(), ModuleError> {
        self.refresh_access_points(ctx);

        Ok(())
    }

    fn view(
        &self,
        data: Self::ViewData<'_>
    ) -> Option<(iced::Element<'static, M>, Option<OnModulePress<M>>)> {
        self.control_center_view(data)
    }
}
