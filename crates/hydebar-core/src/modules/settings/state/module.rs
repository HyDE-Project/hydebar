//! Module trait wiring for the settings menu.

use super::super::{
    Message, Settings,
    event_forwarders::{
        AudioEventForwarder, BluetoothEventForwarder, BrightnessEventForwarder,
        NetworkEventForwarder, UPowerEventForwarder
    },
    view::SettingsViewExt
};
use crate::{
    ModuleContext,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError, OnModulePress},
    services::{
        audio::AudioService, bluetooth::BluetoothService, brightness::BrightnessService,
        network::NetworkService, upower::UPowerService
    }
};

impl<M> Module<M> for Settings
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = <Self as SettingsViewExt>::ViewData<'a>;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        for task in self.tasks.drain(..) {
            task.abort();
        }

        let sender = ctx.module_sender(ModuleEvent::Settings);

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

    fn view(
        &self,
        data: Self::ViewData<'_>
    ) -> Option<(iced::Element<'static, M>, Option<OnModulePress<M>>)> {
        self.settings_view(data)
    }
}
