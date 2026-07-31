mod commands {
    use log::warn;
    use tokio::runtime::Handle;

    use super::{
        audio::AudioMessage,
        bluetooth::BluetoothMessage,
        brightness::BrightnessMessage,
        network::NetworkMessage,
        state::{ControlCenter, Message},
        upower::UPowerMessage
    };
    use crate::services::{
        ReadOnlyService, ServiceEvent,
        audio::{AudioCommand, AudioService},
        bluetooth::{BluetoothCommand, BluetoothService},
        brightness::{BrightnessCommand, BrightnessService},
        network::{NetworkCommand, NetworkService},
        upower::{PowerProfileCommand, UPowerService}
    };

    pub(super) trait ControlCenterCommandExt {
        fn spawn_audio_command(&self, command: AudioCommand) -> bool;
        fn spawn_brightness_command(&self, command: BrightnessCommand) -> bool;
        fn spawn_network_command(&self, command: NetworkCommand) -> bool;
        fn spawn_bluetooth_command(&self, command: BluetoothCommand) -> bool;
        fn spawn_upower_command(&self, command: PowerProfileCommand) -> bool;
    }

    impl ControlCenterCommandExt for ControlCenter {
        fn spawn_audio_command(&self, command: AudioCommand) -> bool {
            spawn_optional_event_command(OptionalEventCommandParams {
                runtime: self.runtime(),
                sender: self.sender(),
                service: self.audio.clone(),
                command,
                runner: AudioService::run_command,
                message_ctor: Message::Audio,
                event_ctor: AudioMessage::Event,
                service_name: "audio"
            })
        }

        fn spawn_brightness_command(&self, command: BrightnessCommand) -> bool {
            spawn_event_command(EventCommandParams {
                runtime: self.runtime(),
                sender: self.sender(),
                service: self.brightness.clone(),
                command,
                runner: BrightnessService::run_command,
                message_ctor: Message::Brightness,
                event_ctor: BrightnessMessage::Event,
                service_name: "brightness"
            })
        }

        fn spawn_network_command(&self, command: NetworkCommand) -> bool {
            spawn_event_command(EventCommandParams {
                runtime: self.runtime(),
                sender: self.sender(),
                service: self.network.clone(),
                command,
                runner: NetworkService::run_command,
                message_ctor: Message::Network,
                event_ctor: NetworkMessage::Event,
                service_name: "network"
            })
        }

        fn spawn_bluetooth_command(&self, command: BluetoothCommand) -> bool {
            spawn_optional_event_command(OptionalEventCommandParams {
                runtime: self.runtime(),
                sender: self.sender(),
                service: self.bluetooth.clone(),
                command,
                runner: BluetoothService::run_command,
                message_ctor: Message::Bluetooth,
                event_ctor: BluetoothMessage::Event,
                service_name: "bluetooth"
            })
        }

        fn spawn_upower_command(&self, command: PowerProfileCommand) -> bool {
            spawn_event_command(EventCommandParams {
                runtime: self.runtime(),
                sender: self.sender(),
                service: self.upower.clone(),
                command,
                runner: UPowerService::run_command,
                message_ctor: Message::UPower,
                event_ctor: UPowerMessage::Event,
                service_name: "upower"
            })
        }
    }

    struct EventCommandParams<S, Command, Fut, Msg>
    where
        S: Send + Clone + ReadOnlyService + 'static,
        Command: Send + 'static,
        Fut: std::future::Future<Output = ServiceEvent<S>> + Send + 'static,
        Msg: Send + 'static
    {
        runtime:      Option<Handle>,
        sender:       Option<crate::ModuleEventSender<Message>>,
        service:      Option<S>,
        command:      Command,
        runner:       fn(S, Command) -> Fut,
        message_ctor: fn(Msg) -> Message,
        event_ctor:   fn(ServiceEvent<S>) -> Msg,
        service_name: &'static str
    }

    fn spawn_event_command<S, Command, Fut, Msg>(
        params: EventCommandParams<S, Command, Fut, Msg>
    ) -> bool
    where
        S: Send + Clone + ReadOnlyService + 'static,
        Command: Send + 'static,
        Fut: std::future::Future<Output = ServiceEvent<S>> + Send + 'static,
        Msg: Send + 'static
    {
        if let (Some(handle), Some(sender), Some(service)) =
            (params.runtime, params.sender, params.service)
        {
            let service_name = params.service_name.to_string();
            let runner = params.runner;
            let message_ctor = params.message_ctor;
            let event_ctor = params.event_ctor;
            let command = params.command;
            handle.spawn(async move {
                let event = runner(service, command).await;
                if let Err(err) = sender.try_send(message_ctor(event_ctor(event))) {
                    warn!("failed to publish {service_name} command event: {err}");
                }
            });
            true
        } else {
            warn!(
                "{} command ignored because runtime, sender, or service is unavailable",
                params.service_name
            );
            false
        }
    }

    struct OptionalEventCommandParams<S, Command, Fut, Msg>
    where
        S: Send + Clone + ReadOnlyService + 'static,
        Command: Send + 'static,
        Fut: std::future::Future<Output = Option<ServiceEvent<S>>> + Send + 'static,
        Msg: Send + 'static
    {
        runtime:      Option<Handle>,
        sender:       Option<crate::ModuleEventSender<Message>>,
        service:      Option<S>,
        command:      Command,
        runner:       fn(S, Command) -> Fut,
        message_ctor: fn(Msg) -> Message,
        event_ctor:   fn(ServiceEvent<S>) -> Msg,
        service_name: &'static str
    }

    fn spawn_optional_event_command<S, Command, Fut, Msg>(
        params: OptionalEventCommandParams<S, Command, Fut, Msg>
    ) -> bool
    where
        S: Send + Clone + ReadOnlyService + 'static,
        Command: Send + 'static,
        Fut: std::future::Future<Output = Option<ServiceEvent<S>>> + Send + 'static,
        Msg: Send + 'static
    {
        if let (Some(handle), Some(sender), Some(service)) =
            (params.runtime, params.sender, params.service)
        {
            let service_name = params.service_name.to_string();
            let runner = params.runner;
            let message_ctor = params.message_ctor;
            let event_ctor = params.event_ctor;
            let command = params.command;
            handle.spawn(async move {
                if let Some(event) = runner(service, command).await
                    && let Err(err) = sender.try_send(message_ctor(event_ctor(event)))
                {
                    warn!("failed to publish {service_name} command event: {err}");
                }
            });
            true
        } else {
            warn!(
                "{} command ignored because runtime, sender, or service is unavailable",
                params.service_name
            );
            false
        }
    }

    // TODO: Fix broken tests
    #[cfg(all(test, feature = "enable-broken-tests"))]
    mod tests {
        use super::*;

        #[test]
        fn commands_fail_gracefully_without_runtime() {
            let settings = ControlCenter::default();

            assert!(!settings.spawn_audio_command(AudioCommand::ToggleSinkMute));
            assert!(!settings.spawn_bluetooth_command(BluetoothCommand::Toggle));
            assert!(!settings.spawn_brightness_command(BrightnessCommand::Set(50)));
            assert!(!settings.spawn_network_command(NetworkCommand::ToggleWiFi));
            assert!(!settings.spawn_upower_command(PowerProfileCommand::Toggle));
        }
    }
}
mod event_forwarders {
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
            |event| Message::Audio(AudioMessage::Event(event)),
            "audio"
        )
    }

    /// The forwarder of the brightness service.
    pub(super) fn brightness_forwarder(
        sender: ModuleEventSender<Message>
    ) -> EventForwarder<BrightnessService> {
        EventForwarder::new(
            sender,
            |event| Message::Brightness(BrightnessMessage::Event(event)),
            "brightness"
        )
    }

    /// The forwarder of the network service.
    pub(super) fn network_forwarder(
        sender: ModuleEventSender<Message>
    ) -> EventForwarder<NetworkService> {
        EventForwarder::new(
            sender,
            |event| Message::Network(NetworkMessage::Event(event)),
            "network"
        )
    }

    /// The forwarder of the bluetooth service.
    pub(super) fn bluetooth_forwarder(
        sender: ModuleEventSender<Message>
    ) -> EventForwarder<BluetoothService> {
        EventForwarder::new(
            sender,
            |event| Message::Bluetooth(BluetoothMessage::Event(event)),
            "bluetooth"
        )
    }

    /// The forwarder of the power service.
    pub(super) fn upower_forwarder(
        sender: ModuleEventSender<Message>
    ) -> EventForwarder<UPowerService> {
        EventForwarder::new(
            sender,
            |event| Message::UPower(UPowerMessage::Event(event)),
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
                    AudioMessage::Event(ServiceEvent::Error(()))
                )))) => {}
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
                    NetworkMessage::Event(ServiceEvent::Error(received))
                )))) => {
                    assert_eq!(received.message(), error.message());
                }
                other => panic!("unexpected event: {other:?}")
            }

            drop(runtime);
        }
    }
}
mod state {
    use tokio::{runtime::Handle, task::JoinHandle};

    use crate::{
        ModuleEventSender,
        services::{
            audio::AudioService, bluetooth::BluetoothService, brightness::BrightnessService,
            idle_inhibitor::IdleInhibitorManager, network::NetworkService, upower::UPowerService
        }
    };

    pub struct ControlCenter {
        pub(super) audio:           Option<AudioService>,
        pub brightness:             Option<BrightnessService>,
        pub(super) network:         Option<NetworkService>,
        pub(super) bluetooth:       Option<BluetoothService>,
        pub(super) idle_inhibitor:  Option<IdleInhibitorManager>,
        pub sub_menu:               Option<SubMenu>,
        pub(super) upower:          Option<UPowerService>,
        pub(super) password_dialog: Option<(String, String)>,
        pub(super) sender:          Option<ModuleEventSender<Message>>,
        pub(super) runtime:         Option<Handle>,
        pub(super) tasks:           Vec<JoinHandle<()>>,
        pub(super) idle_release:    Option<JoinHandle<()>>,
        /// Read of the nearby networks the attended menu asked for.
        ///
        /// Held so a read still in flight is not started a second time: the
        /// answer takes a bus round trip per access point, which on a
        /// busy band outlasts the cadence the menu refreshes at.
        pub(super) network_poll:    Option<JoinHandle<()>>
    }

    impl ControlCenter {
        /// Whether the shared idle inhibitor currently keeps the session awake.
        ///
        /// Returns `false` when the compositor refused the inhibitor protocol,
        /// so callers render the idle state instead of failing.
        #[must_use]
        pub fn is_idle_inhibited(&self) -> bool {
            self.idle_inhibitor
                .as_ref()
                .is_some_and(IdleInhibitorManager::is_inhibited)
        }

        /// Brings the shared inhibitor to `inhibited`, doing nothing when it is
        /// already there or when the compositor refused the protocol.
        ///
        /// Any pending self release is dropped, so a manual toggle always wins
        /// over a timeout armed by an earlier activation.
        pub fn set_idle_inhibited(&mut self, inhibited: bool) {
            if let Some(release) = self.idle_release.take() {
                release.abort();
            }

            let Some(manager) = self.idle_inhibitor.as_mut() else {
                return;
            };

            if manager.is_inhibited() != inhibited {
                manager.toggle();
            }
        }
    }

    impl Default for ControlCenter {
        fn default() -> Self {
            let idle_inhibitor = match IdleInhibitorManager::new() {
                Ok(manager) => Some(manager),
                Err(err) => {
                    log::warn!("Failed to initialize idle inhibitor: {err}");
                    None
                }
            };

            Self {
                audio: None,
                brightness: None,
                network: None,
                bluetooth: None,
                idle_inhibitor,
                sub_menu: None,
                upower: None,
                password_dialog: None,
                sender: None,
                runtime: None,
                tasks: Vec::new(),
                idle_release: None,
                network_poll: None
            }
        }
    }

    mod messages {
        //! Messages and submenu identifiers of the settings module.

        use super::super::{
            audio::AudioMessage, bluetooth::BluetoothMessage, brightness::BrightnessMessage,
            network::NetworkMessage, power::PowerMessage, upower::UPowerMessage
        };
        use crate::password_dialog;

        #[derive(Debug, Clone)]
        pub enum Message {
            ToggleMenu(iced::SurfaceId, crate::position_button::ButtonUIRef),
            UPower(UPowerMessage),
            Network(NetworkMessage),
            Bluetooth(BluetoothMessage),
            Audio(AudioMessage),
            Brightness(BrightnessMessage),
            ToggleInhibitIdle,
            /// Releases the inhibitor the configured timeout has outlived.
            ReleaseInhibitIdle,
            Lock,
            Power(PowerMessage),
            ToggleSubMenu(SubMenu),
            PasswordDialog(password_dialog::Message)
        }

        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        pub enum SubMenu {
            Power,
            Sinks,
            Sources,
            Wifi,
            Vpn,
            Bluetooth
        }
    }
    mod module {
        //! Module trait wiring for the settings menu.

        use std::time::Duration;

        use log::warn;

        use super::super::{
            ControlCenter, Message,
            event_forwarders::{
                audio_forwarder, bluetooth_forwarder, brightness_forwarder, network_forwarder,
                upower_forwarder
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
            /// A list identical to the one already on screen is dropped rather
            /// than published: every event reaching the bar
            /// rebuilds every surface it owns, and a band nobody is
            /// roaming reports the same radios sample after sample.
            pub(crate) fn refresh_access_points(&mut self, ctx: &ModuleContext) {
                if self
                    .network_poll
                    .as_ref()
                    .is_some_and(|poll| !poll.is_finished())
                {
                    return;
                }

                let (Some(service), Some(sender)) = (self.network.as_ref(), self.sender.clone())
                else {
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

                            if let Err(err) = sender.try_send(Message::Network(
                                NetworkMessage::Event(ServiceEvent::Update(
                                    NetworkEvent::WirelessAccessPoint(access_points)
                                ))
                            )) {
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

                let mut audio_publisher = audio_forwarder(sender.clone());
                tasks.push(ctx.runtime_handle().spawn(async move {
                    AudioService::listen(&mut audio_publisher).await;
                }));

                let mut brightness_publisher = brightness_forwarder(sender.clone());
                tasks.push(ctx.runtime_handle().spawn(async move {
                    BrightnessService::listen(&mut brightness_publisher).await;
                }));

                let mut network_publisher = network_forwarder(sender.clone());
                tasks.push(ctx.runtime_handle().spawn(async move {
                    NetworkService::listen(&mut network_publisher).await;
                }));

                let mut bluetooth_publisher = bluetooth_forwarder(sender.clone());
                tasks.push(ctx.runtime_handle().spawn(async move {
                    BluetoothService::listen(&mut bluetooth_publisher).await;
                }));

                let mut upower_publisher = upower_forwarder(sender.clone());
                tasks.push(ctx.runtime_handle().spawn(async move {
                    UPowerService::listen(&mut upower_publisher).await;
                }));

                self.sender = Some(sender);
                self.runtime = Some(ctx.runtime_handle().clone());
                self.tasks = tasks;

                Ok(())
            }

            /// Disconnects the five hardware services once nothing on the bar
            /// shows them.
            ///
            /// Audio, brightness, network, bluetooth and UPower each hold a
            /// D-Bus or PulseAudio connection that reports on every
            /// volume step, every signal strength sample and every
            /// battery reading. Together they are the single
            /// largest idle cost the bar can carry, and a layout without any of
            /// their readouts pays it for nothing.
            ///
            /// The idle inhibitor is untouched: it belongs to the compositor
            /// session rather than to a service, and the module
            /// keeps rendering its state.
            fn deregister(&mut self) {
                for task in self.tasks.drain(..) {
                    task.abort();
                }

                if let Some(poll) = self.network_poll.take() {
                    poll.abort();
                }

                self.sender = None;
            }

            /// Refreshes the nearby networks only while the menu showing them
            /// is attended.
            ///
            /// Nothing on the bar draws the list, so a resting cadence would
            /// pay a bus round trip per neighbouring radio for a
            /// readout behind a closed menu.
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
    }
    mod update {
        //! Handling of settings menu messages.

        use log::{info, warn};
        use tokio::runtime::Handle;

        use super::super::{
            ControlCenter, Message, SubMenu, audio::AudioMessage, bluetooth::BluetoothMessage,
            brightness::BrightnessMessage, commands::ControlCenterCommandExt,
            network::NetworkMessage, upower::UPowerMessage
        };
        use crate::{
            ModuleEventSender,
            config::ControlCenterModuleConfig,
            menu::MenuType,
            outputs::Outputs,
            password_dialog,
            services::{
                ReadOnlyService, ServiceEvent,
                audio::AudioCommand,
                bluetooth::BluetoothCommand,
                brightness::BrightnessCommand,
                network::{NetworkCommand, NetworkEvent},
                upower::PowerProfileCommand
            }
        };

        /// Volume moved by one wheel notch over the bar entry, in percent.
        const WHEEL_VOLUME_STEP: i32 = 5;

        impl ControlCenter {
            pub(crate) fn runtime(&self) -> Option<Handle> {
                self.runtime.as_ref().cloned()
            }

            pub(crate) fn sender(&self) -> Option<ModuleEventSender<Message>> {
                self.sender.as_ref().cloned()
            }

            /// Schedules the release of an activation that outlives `delay`.
            ///
            /// Without a delay the inhibitor stays until it is toggled off,
            /// which is what a configuration naming no timeout asks
            /// for.
            fn arm_idle_release(&mut self, delay: Option<std::time::Duration>) {
                let (Some(delay), Some(runtime), Some(sender)) =
                    (delay, self.runtime(), self.sender())
                else {
                    return;
                };

                self.idle_release = Some(runtime.spawn(async move {
                    tokio::time::sleep(delay).await;

                    if let Err(err) = sender.try_send(Message::ReleaseInhibitIdle) {
                        warn!("failed to release the idle inhibitor after its timeout: {err}");
                    }
                }));
            }

            pub fn update(
                &mut self,
                message: Message,
                config: &ControlCenterModuleConfig,
                outputs: &mut Outputs,
                main_config: &crate::config::Config
            ) {
                match message {
                    Message::ToggleMenu(id, button_ui_ref) => {
                        self.sub_menu = None;
                        self.password_dialog = None;
                        let _ = outputs.toggle_menu::<Message>(
                            id,
                            MenuType::ControlCenter,
                            button_ui_ref,
                            main_config
                        );
                    }
                    Message::Audio(msg) => match msg {
                        AudioMessage::Event(event) => match event {
                            ServiceEvent::Init(service) => {
                                self.audio = Some(service);
                            }
                            ServiceEvent::Update(data) => {
                                if let Some(audio) = self.audio.as_mut() {
                                    audio.update(data);

                                    if self.sub_menu == Some(SubMenu::Sinks)
                                        && audio.sinks.len() < 2
                                    {
                                        self.sub_menu = None;
                                    }

                                    if self.sub_menu == Some(SubMenu::Sources)
                                        && audio.sources.len() < 2
                                    {
                                        self.sub_menu = None;
                                    }
                                }
                            }
                            ServiceEvent::Error(err) => {
                                log::error!("Audio service error: {err:?}");
                            }
                        },
                        AudioMessage::ToggleSinkMute => {
                            let _spawned = self.spawn_audio_command(AudioCommand::ToggleSinkMute);
                        }
                        AudioMessage::SinkVolumeChanged(value) => {
                            let _spawned =
                                self.spawn_audio_command(AudioCommand::SinkVolume(value));
                        }
                        AudioMessage::SinkVolumeWheel(direction) => {
                            if let Some(audio) = self.audio.as_ref() {
                                let value = (audio.cur_sink_volume
                                    + direction * WHEEL_VOLUME_STEP)
                                    .clamp(0, 100);

                                let _spawned =
                                    self.spawn_audio_command(AudioCommand::SinkVolume(value));
                            }
                        }
                        AudioMessage::DefaultSinkChanged(name, port) => {
                            let _spawned =
                                self.spawn_audio_command(AudioCommand::DefaultSink(name, port));
                        }
                        AudioMessage::ToggleSourceMute => {
                            let _spawned =
                                self.spawn_audio_command(AudioCommand::ToggleSourceMute);
                        }
                        AudioMessage::SourceVolumeChanged(value) => {
                            let _spawned =
                                self.spawn_audio_command(AudioCommand::SourceVolume(value));
                        }
                        AudioMessage::DefaultSourceChanged(name, port) => {
                            let _spawned =
                                self.spawn_audio_command(AudioCommand::DefaultSource(name, port));
                        }
                        AudioMessage::SinksMore(id) => {
                            if let Some(cmd) = &config.audio_sinks_more_cmd {
                                crate::utils::launcher::execute_command(cmd.to_string());
                                let _ = outputs.close_menu::<Message>(id, main_config);
                            }
                        }
                        AudioMessage::SourcesMore(id) => {
                            if let Some(cmd) = &config.audio_sources_more_cmd {
                                crate::utils::launcher::execute_command(cmd.to_string());
                                let _ = outputs.close_menu::<Message>(id, main_config);
                            }
                        }
                    },
                    Message::UPower(msg) => match msg {
                        UPowerMessage::Event(event) => match event {
                            ServiceEvent::Init(service) => {
                                self.upower = Some(service);
                            }
                            ServiceEvent::Update(data) => {
                                if let Some(upower) = self.upower.as_mut() {
                                    upower.update(data);
                                }
                            }
                            ServiceEvent::Error(err) => {
                                log::error!("UPower service error: {err:?}");
                            }
                        },
                        UPowerMessage::TogglePowerProfile => {
                            let _spawned = self.spawn_upower_command(PowerProfileCommand::Toggle);
                        }
                    },
                    Message::Network(msg) => match msg {
                        NetworkMessage::Event(event) => match event {
                            ServiceEvent::Init(service) => {
                                self.network = Some(service);
                            }
                            ServiceEvent::Update(NetworkEvent::RequestPasswordForSSID(ssid)) => {
                                self.password_dialog = Some((ssid, String::new()));
                            }
                            ServiceEvent::Update(data) => {
                                if let Some(network) = self.network.as_mut() {
                                    network.update(data);
                                }
                            }
                            ServiceEvent::Error(err) => {
                                log::error!("Network service error: {err:?}");
                            }
                        },
                        NetworkMessage::ToggleAirplaneMode => {
                            if self.sub_menu == Some(SubMenu::Wifi) {
                                self.sub_menu = None;
                            }

                            let _spawned =
                                self.spawn_network_command(NetworkCommand::ToggleAirplaneMode);
                        }
                        NetworkMessage::ToggleWiFi => {
                            if self.sub_menu == Some(SubMenu::Wifi) {
                                self.sub_menu = None;
                            }

                            let _spawned = self.spawn_network_command(NetworkCommand::ToggleWiFi);
                        }
                        NetworkMessage::SelectAccessPoint(ac) => {
                            let _spawned = self.spawn_network_command(
                                NetworkCommand::SelectAccessPoint((ac, None))
                            );
                        }
                        NetworkMessage::RequestWiFiPassword(id, ssid) => {
                            info!("Requesting password for {ssid}");
                            self.password_dialog = Some((ssid, String::new()));
                            let _ = outputs
                                .request_keyboard::<Message>(id, main_config.menu_keyboard_focus);
                        }
                        NetworkMessage::ScanNearByWiFi => {
                            let _spawned =
                                self.spawn_network_command(NetworkCommand::ScanNearByWiFi);
                        }
                        NetworkMessage::WiFiMore(id) => {
                            if let Some(cmd) = &config.wifi_more_cmd {
                                crate::utils::launcher::execute_command(cmd.to_string());
                                let _ = outputs.close_menu::<Message>(id, main_config);
                            }
                        }
                        NetworkMessage::VpnMore(id) => {
                            if let Some(cmd) = &config.vpn_more_cmd {
                                crate::utils::launcher::execute_command(cmd.to_string());
                                let _ = outputs.close_menu::<Message>(id, main_config);
                            }
                        }
                        NetworkMessage::ToggleVpn(vpn) => {
                            let _spawned =
                                self.spawn_network_command(NetworkCommand::ToggleVpn(vpn));
                        }
                    },
                    Message::Bluetooth(msg) => match msg {
                        BluetoothMessage::Event(event) => match event {
                            ServiceEvent::Init(service) => {
                                self.bluetooth = Some(service);
                            }
                            ServiceEvent::Update(data) => {
                                if let Some(bluetooth) = self.bluetooth.as_mut() {
                                    bluetooth.update(data);
                                }
                            }
                            ServiceEvent::Error(err) => {
                                log::error!("Bluetooth service error: {err:?}");
                            }
                        },
                        BluetoothMessage::Toggle => match self.bluetooth.as_mut() {
                            Some(_) => {
                                if self.sub_menu == Some(SubMenu::Bluetooth) {
                                    self.sub_menu = None;
                                }

                                let _spawned =
                                    self.spawn_bluetooth_command(BluetoothCommand::Toggle);
                            }
                            None => {
                                log::warn!("Bluetooth service not initialized");
                            }
                        },
                        BluetoothMessage::ConnectDevice(device_path) => {
                            let _spawned = self.spawn_bluetooth_command(
                                BluetoothCommand::ConnectDevice(device_path)
                            );
                        }
                        BluetoothMessage::DisconnectDevice(device_path) => {
                            let _spawned = self.spawn_bluetooth_command(
                                BluetoothCommand::DisconnectDevice(device_path)
                            );
                        }
                        BluetoothMessage::More(id) => {
                            if let Some(cmd) = &config.bluetooth_more_cmd {
                                crate::utils::launcher::execute_command(cmd.to_string());
                                let _ = outputs.close_menu::<Message>(id, main_config);
                            }
                        }
                    },
                    Message::Brightness(msg) => match msg {
                        BrightnessMessage::Event(event) => match event {
                            ServiceEvent::Init(service) => {
                                self.brightness = Some(service);
                            }
                            ServiceEvent::Update(data) => {
                                if let Some(brightness) = self.brightness.as_mut() {
                                    brightness.update(data);
                                }
                            }
                            ServiceEvent::Error(err) => {
                                log::error!("Brightness service error: {err:?}");
                            }
                        },
                        BrightnessMessage::Change(value) => {
                            let _spawned =
                                self.spawn_brightness_command(BrightnessCommand::Set(value));
                        }
                    },
                    Message::ToggleSubMenu(menu_type) => {
                        if self.sub_menu == Some(menu_type) {
                            self.sub_menu.take();
                        } else {
                            self.sub_menu.replace(menu_type);

                            if menu_type == SubMenu::Wifi {
                                let _spawned =
                                    self.spawn_network_command(NetworkCommand::ScanNearByWiFi);
                            }
                        }
                    }
                    Message::ToggleInhibitIdle => {
                        let inhibited = self.is_idle_inhibited();
                        self.set_idle_inhibited(!inhibited);

                        if self.is_idle_inhibited() {
                            self.arm_idle_release(main_config.idle_inhibitor.release_after());
                        }
                    }
                    Message::ReleaseInhibitIdle => {
                        self.set_idle_inhibited(false);
                    }
                    Message::Lock => {
                        if let Some(lock_cmd) = &config.lock_cmd {
                            crate::utils::launcher::execute_command(lock_cmd.to_string());
                        }
                    }
                    Message::Power(msg) => {
                        msg.update();
                    }
                    Message::PasswordDialog(msg) => match msg {
                        password_dialog::Message::PasswordChanged(password) => {
                            if let Some((_, current_password)) = &mut self.password_dialog {
                                *current_password = password;
                            }
                        }
                        password_dialog::Message::DialogConfirmed(id) => {
                            if let Some((ssid, password)) = self.password_dialog.take() {
                                if let Some(network) = self.network.as_ref()
                                    && let Some(access_point) = network
                                        .wireless_access_points
                                        .iter()
                                        .find(|ap| ap.ssid == ssid)
                                        .cloned()
                                {
                                    self.spawn_network_command(NetworkCommand::SelectAccessPoint(
                                        (
                                            // We intentionally clone the password to avoid
                                            // holding a
                                            // mutable reference across the async boundary.
                                            access_point,
                                            Some(password.clone())
                                        )
                                    ));
                                }

                                let _ = outputs.release_keyboard::<Message>(
                                    id,
                                    main_config.menu_keyboard_focus
                                );
                            } else {
                                let _ = outputs.release_keyboard::<Message>(
                                    id,
                                    main_config.menu_keyboard_focus
                                );
                            }
                        }
                        password_dialog::Message::DialogCancelled(id) => {
                            self.password_dialog = None;

                            let _ = outputs
                                .release_keyboard::<Message>(id, main_config.menu_keyboard_focus);
                        }
                    }
                }
            }
        }
    }

    #[cfg(all(test, feature = "enable-broken-tests"))]
    mod tests {
        // TODO: Fix broken tests
        #[cfg(all(test, feature = "enable-broken-tests"))]
        mod tests {
            use std::{
                num::NonZeroUsize,
                sync::{
                    Arc,
                    atomic::{AtomicBool, Ordering}
                }
            };

            use futures::future;
            use tokio::runtime::Runtime;

            use super::*;
            use crate::{event_bus::EventBus, modules::Module};

            #[test]
            fn register_spawns_event_forwarders() {
                let runtime = Runtime::new().expect("runtime");
                let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
                let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
                let mut settings = ControlCenter::default();

                <ControlCenter as Module<Message>>::register(&mut settings, &ctx, ())
                    .expect("register should succeed");

                assert!(settings.sender.is_some());
                assert!(settings.runtime.is_some());
                assert_eq!(settings.tasks.len(), 5);

                for task in settings.tasks.drain(..) {
                    task.abort();
                }
            }

            #[test]
            #[ignore = "Timing-sensitive test - needs rework"]
            fn register_aborts_existing_tasks() {
                let runtime = Runtime::new().expect("runtime");
                let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
                let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
                let mut settings = ControlCenter::default();

                let cancelled = Arc::new(AtomicBool::new(false));
                let guard_flag = Arc::clone(&cancelled);

                settings.tasks.push(runtime.spawn(async move {
                    struct CancelGuard(Arc<AtomicBool>);

                    impl Drop for CancelGuard {
                        fn drop(&mut self) {
                            self.0.store(true, Ordering::SeqCst);
                        }
                    }

                    let _guard = CancelGuard(guard_flag);

                    future::pending::<()>().await;
                }));

                <ControlCenter as Module<Message>>::register(&mut settings, &ctx, ())
                    .expect("register should succeed");

                assert!(cancelled.load(Ordering::SeqCst));

                for task in settings.tasks.drain(..) {
                    task.abort();
                }
            }
        }
    }

    pub use messages::{Message, SubMenu};
}
mod view {
    use iced::{Element, SurfaceId as Id};

    use super::state::{ControlCenter, Message};
    use crate::{
        components::icons::IconTheme,
        config::{ControlCenterModuleConfig, Position},
        modules::OnModulePress
    };

    mod bar {
        //! Rendering of the settings indicator on the bar.

        use iced::{
            Element, Theme,
            widget::{Row, container}
        };

        use crate::{
            components::{
                icons::{IconTheme, Icons, icon},
                push_maybe::PushMaybe,
                scale
            },
            menu::MenuType,
            modules::{
                OnModulePress,
                control_center::state::{ControlCenter, Message}
            }
        };

        impl ControlCenter {
            pub(super) fn render_bar<M>(
                &self,
                icons: &IconTheme
            ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
            where
                M: 'static + From<Message>
            {
                let idle_inhibited = self
                    .idle_inhibitor
                    .as_ref()
                    .map(|i| i.is_inhibited())
                    .unwrap_or(false);
                let power_profile_indicator = self
                    .upower
                    .as_ref()
                    .and_then(|p| p.power_profile.indicator(icons));
                let sink_indicator = self.audio.as_ref().and_then(|a| a.sink_indicator(icons));
                let connection_indicator = self
                    .network
                    .as_ref()
                    .and_then(|n| n.get_connection_indicator(icons));
                let vpn_indicator = self
                    .network
                    .as_ref()
                    .and_then(|n| n.get_vpn_indicator(icons));
                let battery_indicator = self
                    .upower
                    .as_ref()
                    .and_then(|upower| upower.battery)
                    .map(|battery| battery.indicator(icons));

                Some((
                    Row::new()
                        .push_maybe(if idle_inhibited {
                            Some(container(icon(icons, Icons::EyeOpened)).style(
                                |theme: &Theme| container::Style {
                                    text_color: Some(theme.palette().danger),
                                    ..Default::default()
                                }
                            ))
                        } else {
                            None
                        })
                        .push_maybe(power_profile_indicator)
                        .push_maybe(sink_indicator)
                        .push(
                            Row::new()
                                .push_maybe(connection_indicator)
                                .push_maybe(vpn_indicator)
                                .spacing(scale::icon_gap())
                        )
                        .push_maybe(battery_indicator)
                        .spacing(scale::item_gap())
                        .into(),
                    Some(OnModulePress::ToggleMenu(MenuType::ControlCenter))
                ))
            }
        }
    }
    mod helpers {
        //! Shared layout helpers for the settings menu.

        use iced::{
            Background, Border, Element, Length, Theme,
            widget::{Space, column, container, row}
        };

        use crate::{
            components::scale, modules::control_center::state::Message, style::darken_color
        };

        /// How much darker an unfolded zone is than the menu it sits in.
        ///
        /// Darker rather than lighter on purpose: the zone is a recess the
        /// extra facts sit in, and a lighter shade reads as a raised
        /// control instead.
        const SUB_MENU_DARKENING: f32 = 0.25;

        pub(super) fn quick_settings_section<'a>(
            buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>,
            opacity: f32
        ) -> Element<'a, Message> {
            let mut section = column!().width(Length::Fill).spacing(scale::scaled(8.0));

            let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

            for (button, menu) in buttons.into_iter() {
                match before.take() {
                    Some((before_button, before_menu)) => {
                        section = section.push(
                            row![before_button, button]
                                .width(Length::Fill)
                                .spacing(scale::scaled(8.0))
                        );

                        if let Some(menu) = before_menu {
                            section = section.push(sub_menu_wrapper(menu, opacity));
                        }

                        if let Some(menu) = menu {
                            section = section.push(sub_menu_wrapper(menu, opacity));
                        }
                    }
                    _ => {
                        before = Some((button, menu));
                    }
                }
            }

            if let Some((before_button, before_menu)) = before.take() {
                section = section.push(
                    row![before_button, Space::new().width(Length::Fill)]
                        .width(Length::Fill)
                        .spacing(scale::scaled(8.0))
                );

                if let Some(menu) = before_menu {
                    section = section.push(sub_menu_wrapper(menu, opacity));
                }
            }

            section.into()
        }

        pub(crate) fn sub_menu_wrapper<Msg: 'static>(
            content: Element<Msg>,
            opacity: f32
        ) -> Element<Msg> {
            container(content)
                .style(move |theme: &Theme| container::Style {
                    background: Background::Color(
                        darken_color(theme.palette().background, SUB_MENU_DARKENING)
                            .scale_alpha(opacity)
                    )
                    .into(),
                    border: Border::default().rounded(scale::scaled(16.0)),
                    ..container::Style::default()
                })
                .padding(scale::scaled(16.0))
                .width(Length::Fill)
                .into()
        }
    }
    mod menu {
        //! Rendering of the settings menu contents.

        use iced::{
            Element, Length, SurfaceId as Id,
            widget::{Column, Row, Space, button}
        };

        use super::{
            helpers::{quick_settings_section, sub_menu_wrapper},
            quick_setting_button
        };
        use crate::{
            components::{
                icons::{IconTheme, Icons, icon},
                push_maybe::PushMaybe,
                scale
            },
            config::{ControlCenterModuleConfig, Position},
            modules::control_center::{
                power::power_menu,
                state::{ControlCenter, Message, SubMenu}
            },
            password_dialog,
            services::bluetooth::BluetoothState,
            style::settings_button_style
        };

        impl ControlCenter {
            pub(super) fn render_menu(
                &self,
                id: Id,
                config: &ControlCenterModuleConfig,
                opacity: f32,
                position: Position,
                icons: &IconTheme
            ) -> Element<'_, Message> {
                if let Some((ssid, current_password)) = &self.password_dialog {
                    password_dialog::view(id, ssid, current_password, opacity, icons)
                        .map(Message::PasswordDialog)
                } else {
                    let battery_data = self
                        .upower
                        .as_ref()
                        .and_then(|upower| upower.battery)
                        .map(|battery| battery.settings_indicator(icons));
                    let right_buttons = Row::new()
                        .push_maybe(config.lock_cmd.as_ref().map(|_| {
                            button(icon(icons, Icons::Lock))
                                .padding([scale::scaled(8.0), scale::scaled(13.0)])
                                .on_press(Message::Lock)
                                .style(settings_button_style(opacity))
                        }))
                        .push(
                            button(icon(
                                icons,
                                if self.sub_menu == Some(SubMenu::Power) {
                                    Icons::Close
                                } else {
                                    Icons::Power
                                }
                            ))
                            .padding([scale::scaled(8.0), scale::scaled(13.0)])
                            .on_press(Message::ToggleSubMenu(SubMenu::Power))
                            .style(settings_button_style(opacity))
                        )
                        .spacing(scale::scaled(8.0));

                    let header = Row::new()
                        .push_maybe(battery_data)
                        .push(Space::new().width(Length::Fill))
                        .push(right_buttons)
                        .spacing(scale::scaled(8.0))
                        .width(Length::Fill);

                    let (sink_slider, source_slider) = self
                        .audio
                        .as_ref()
                        .map(|a| a.audio_sliders(self.sub_menu, opacity, icons))
                        .unwrap_or((None, None));

                    let wifi_setting_button = self.network.as_ref().and_then(|n| {
                        n.get_wifi_quick_setting_button(
                            id,
                            self.sub_menu,
                            config.wifi_more_cmd.is_some(),
                            opacity,
                            icons
                        )
                    });
                    let quick_settings = quick_settings_section(
                        vec![
                            wifi_setting_button,
                            self.bluetooth
                                .as_ref()
                                .filter(|b| b.state != BluetoothState::Unavailable)
                                .and_then(|b| {
                                    b.get_quick_setting_button(
                                        id,
                                        self.sub_menu,
                                        config.bluetooth_more_cmd.is_some(),
                                        opacity,
                                        icons
                                    )
                                }),
                            self.network.as_ref().and_then(|n| {
                                n.get_vpn_quick_setting_button(
                                    id,
                                    self.sub_menu,
                                    config.vpn_more_cmd.is_some(),
                                    opacity,
                                    icons
                                )
                            }),
                            self.network.as_ref().and_then(|n| {
                                if config.remove_airplane_btn {
                                    None
                                } else {
                                    Some(n.get_airplane_mode_quick_setting_button(opacity, icons))
                                }
                            }),
                            self.idle_inhibitor.as_ref().and_then(|i| {
                                if config.remove_idle_btn {
                                    None
                                } else {
                                    Some((
                                        quick_setting_button(
                                            icons,
                                            if i.is_inhibited() {
                                                Icons::EyeOpened
                                            } else {
                                                Icons::EyeClosed
                                            },
                                            "Idle Inhibitor".to_string(),
                                            None,
                                            i.is_inhibited(),
                                            Message::ToggleInhibitIdle,
                                            None,
                                            opacity
                                        ),
                                        None
                                    ))
                                }
                            }),
                            self.upower.as_ref().and_then(|u| {
                                u.power_profile.get_quick_setting_button(opacity, icons)
                            }),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                        opacity
                    );

                    let (top_sink_slider, bottom_sink_slider) = match position {
                        Position::Top => (sink_slider, None),
                        Position::Bottom => (None, sink_slider)
                    };
                    let (top_source_slider, bottom_source_slider) = match position {
                        Position::Top => (source_slider, None),
                        Position::Bottom => (None, source_slider)
                    };

                    Column::new()
                        .push(header)
                        .push_maybe(
                            self.sub_menu
                                .filter(|menu_type| *menu_type == SubMenu::Power)
                                .map(|_| {
                                    sub_menu_wrapper(
                                        power_menu(opacity, config, icons).map(Message::Power),
                                        opacity
                                    )
                                })
                        )
                        .push_maybe(top_sink_slider)
                        .push_maybe(
                            self.sub_menu
                                .filter(|menu_type| *menu_type == SubMenu::Sinks)
                                .and_then(|_| {
                                    self.audio.as_ref().map(|a| {
                                        sub_menu_wrapper(
                                            a.sinks_submenu(
                                                id,
                                                config.audio_sinks_more_cmd.is_some(),
                                                opacity,
                                                icons
                                            ),
                                            opacity
                                        )
                                    })
                                })
                        )
                        .push_maybe(bottom_sink_slider)
                        .push_maybe(top_source_slider)
                        .push_maybe(
                            self.sub_menu
                                .filter(|menu_type| *menu_type == SubMenu::Sources)
                                .and_then(|_| {
                                    self.audio.as_ref().map(|a| {
                                        sub_menu_wrapper(
                                            a.sources_submenu(
                                                id,
                                                config.audio_sources_more_cmd.is_some(),
                                                opacity,
                                                icons
                                            ),
                                            opacity
                                        )
                                    })
                                })
                        )
                        .push_maybe(bottom_source_slider)
                        .push_maybe(self.brightness.as_ref().map(|b| b.brightness_slider(icons)))
                        .push(quick_settings)
                        .width(Length::Fill)
                        .spacing(scale::scaled(16.0))
                        .into()
                }
            }
        }
    }
    mod quick_button {
        //! Quick setting toggle button.

        use iced::{
            Alignment, Element, Length, Padding,
            alignment::{Horizontal, Vertical},
            widget::{Column, Row, button, container, row}
        };

        use crate::{
            components::{
                icons::{IconTheme, Icons, icon},
                push_maybe::PushMaybe,
                scale,
                text::text
            },
            modules::control_center::state::SubMenu,
            style::{quick_settings_button_style, quick_settings_submenu_button_style}
        };

        pub fn quick_setting_button<'a, Msg: Clone + 'static>(
            icons: &IconTheme,
            icon_type: Icons,
            title: String,
            subtitle: Option<String>,
            active: bool,
            on_press: Msg,
            with_submenu: Option<(SubMenu, Option<SubMenu>, Msg)>,
            opacity: f32
        ) -> Element<'a, Msg> {
            let main_content = row!(
                icon(icons, icon_type).size(scale::scaled(20.0)),
                Column::new()
                    .push(text(title).size(scale::scaled(12.0)))
                    .push_maybe(subtitle.map(|s| text(s).size(scale::scaled(10.0))))
                    .spacing(scale::scaled(4.0))
            )
            .spacing(scale::scaled(8.0))
            .padding(Padding::ZERO.left(4))
            .width(Length::Fill)
            .align_y(Alignment::Center);

            button(
                Row::new()
                    .push(main_content)
                    .push_maybe(with_submenu.map(|(menu_type, submenu, msg)| {
                        button(
                            container(icon(
                                icons,
                                if Some(menu_type) == submenu {
                                    Icons::Close
                                } else {
                                    Icons::RightChevron
                                }
                            ))
                            .align_y(Vertical::Center)
                            .align_x(Horizontal::Center)
                        )
                        .padding([
                            scale::scaled(4.0),
                            scale::scaled(if Some(menu_type) == submenu {
                                9.0
                            } else {
                                12.0
                            })
                        ])
                        .style(quick_settings_submenu_button_style(active, opacity))
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .on_press(msg)
                    }))
                    .spacing(scale::scaled(4.0))
                    .align_y(Alignment::Center)
                    .height(Length::Fill)
            )
            .padding([scale::scaled(4.0), scale::scaled(8.0)])
            .on_press(on_press)
            .height(Length::Fill)
            .width(Length::Fill)
            .style(quick_settings_button_style(active, opacity))
            .width(Length::Fill)
            .height(Length::Fixed(scale::scaled(50.0)))
            .into()
        }
    }
    mod standalone {
        //! Bar entries and menus of the modules split out of the control
        //! center.
        //!
        //! Audio, network, bluetooth and the power profile each render their
        //! own bar entry and own menu, while the services behind them
        //! stay in the single [`ControlCenter`] state: splitting the
        //! presentation must not multiply the D-Bus connections the bar
        //! keeps open.

        use iced::{
            Element, Length, SurfaceId as Id,
            widget::{Column, Row, mouse_area}
        };

        use super::{
            helpers::{quick_settings_section, sub_menu_wrapper},
            quick_setting_button
        };
        use crate::{
            components::{
                icons::{IconTheme, Icons, icon},
                push_maybe::PushMaybe,
                scale
            },
            config::{ControlCenterModuleConfig, Position},
            menu::MenuType,
            modules::{
                OnModulePress,
                control_center::{
                    audio::{AudioMessage, wheel_direction},
                    power::power_menu,
                    state::{ControlCenter, Message, SubMenu}
                }
            },
            password_dialog,
            services::bluetooth::BluetoothState
        };

        impl ControlCenter {
            /// Bar entry of the standalone audio module.
            ///
            /// Renders nothing while the audio service is away, so a session
            /// without a sound server keeps a bar free of dead
            /// icons.
            ///
            /// The entry answers the wheel as well: a notch up or down nudges
            /// the sink volume without the menu ever opening, the
            /// way the reference waybar module behaves.
            pub fn audio_bar<M>(
                &self,
                icons: &IconTheme
            ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
            where
                M: 'static + From<Message> + Clone
            {
                let indicator = self.audio.as_ref().and_then(|a| a.sink_indicator(icons))?;
                let wheeled = mouse_area(indicator)
                    .on_scroll(|delta| {
                        M::from(Message::Audio(AudioMessage::SinkVolumeWheel(
                            wheel_direction(delta)
                        )))
                    })
                    .into();

                Some((wheeled, Some(OnModulePress::ToggleMenu(MenuType::Audio))))
            }

            /// Bar entry of the standalone network module, connection and VPN.
            pub fn network_bar<M>(
                &self,
                icons: &IconTheme
            ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
            where
                M: 'static + From<Message>
            {
                let network = self.network.as_ref()?;
                let connection = network.get_connection_indicator(icons);
                let vpn = network.get_vpn_indicator(icons);

                if connection.is_none() && vpn.is_none() {
                    return None;
                }

                Some((
                    Row::new()
                        .push_maybe(connection)
                        .push_maybe(vpn)
                        .spacing(scale::icon_gap())
                        .into(),
                    Some(OnModulePress::ToggleMenu(MenuType::Network))
                ))
            }

            /// Bar entry of the standalone bluetooth module.
            ///
            /// A machine without a bluetooth radio reports the state as
            /// unavailable and the module stays off the bar.
            pub fn bluetooth_bar<M>(
                &self,
                icons: &IconTheme
            ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
            where
                M: 'static + From<Message>
            {
                self.bluetooth
                    .as_ref()
                    .filter(|b| b.state != BluetoothState::Unavailable)?;

                Some((
                    icon(icons, Icons::Bluetooth).into(),
                    Some(OnModulePress::ToggleMenu(MenuType::Bluetooth))
                ))
            }

            /// Bar entry of the standalone power profile module.
            pub fn power_profile_bar<M>(
                &self,
                icons: &IconTheme
            ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
            where
                M: 'static + From<Message>
            {
                let indicator = self
                    .upower
                    .as_ref()
                    .and_then(|p| p.power_profile.indicator(icons))?;

                Some((
                    indicator,
                    Some(OnModulePress::ToggleMenu(MenuType::PowerProfile))
                ))
            }

            /// Menu of the standalone audio module: both sliders and their
            /// device lists.
            pub fn audio_menu(
                &self,
                id: Id,
                config: &ControlCenterModuleConfig,
                opacity: f32,
                position: Position,
                icons: &IconTheme
            ) -> Element<'_, Message> {
                let (sink_slider, source_slider) = self
                    .audio
                    .as_ref()
                    .map(|a| a.audio_sliders(self.sub_menu, opacity, icons))
                    .unwrap_or((None, None));

                let (top_sink_slider, bottom_sink_slider) = match position {
                    Position::Top => (sink_slider, None),
                    Position::Bottom => (None, sink_slider)
                };
                let (top_source_slider, bottom_source_slider) = match position {
                    Position::Top => (source_slider, None),
                    Position::Bottom => (None, source_slider)
                };

                Column::new()
                    .push_maybe(top_sink_slider)
                    .push_maybe(
                        self.sub_menu
                            .filter(|menu_type| *menu_type == SubMenu::Sinks)
                            .and_then(|_| {
                                self.audio.as_ref().map(|a| {
                                    sub_menu_wrapper(
                                        a.sinks_submenu(
                                            id,
                                            config.audio_sinks_more_cmd.is_some(),
                                            opacity,
                                            icons
                                        ),
                                        opacity
                                    )
                                })
                            })
                    )
                    .push_maybe(bottom_sink_slider)
                    .push_maybe(top_source_slider)
                    .push_maybe(
                        self.sub_menu
                            .filter(|menu_type| *menu_type == SubMenu::Sources)
                            .and_then(|_| {
                                self.audio.as_ref().map(|a| {
                                    sub_menu_wrapper(
                                        a.sources_submenu(
                                            id,
                                            config.audio_sources_more_cmd.is_some(),
                                            opacity,
                                            icons
                                        ),
                                        opacity
                                    )
                                })
                            })
                    )
                    .push_maybe(bottom_source_slider)
                    .width(Length::Fill)
                    .spacing(scale::scaled(16.0))
                    .into()
            }

            /// Menu of the standalone network module: connection, VPN and
            /// airplane mode.
            ///
            /// The password prompt of a protected network belongs here as well,
            /// since this is the menu the connection attempt starts
            /// from.
            pub fn network_menu(
                &self,
                id: Id,
                config: &ControlCenterModuleConfig,
                opacity: f32,
                icons: &IconTheme
            ) -> Element<'_, Message> {
                if let Some((ssid, current_password)) = &self.password_dialog {
                    return password_dialog::view(id, ssid, current_password, opacity, icons)
                        .map(Message::PasswordDialog);
                }

                let buttons = vec![
                    self.network.as_ref().and_then(|n| {
                        n.get_wifi_quick_setting_button(
                            id,
                            self.sub_menu,
                            config.wifi_more_cmd.is_some(),
                            opacity,
                            icons
                        )
                    }),
                    self.network.as_ref().and_then(|n| {
                        n.get_vpn_quick_setting_button(
                            id,
                            self.sub_menu,
                            config.vpn_more_cmd.is_some(),
                            opacity,
                            icons
                        )
                    }),
                    self.network.as_ref().and_then(|n| {
                        if config.remove_airplane_btn {
                            None
                        } else {
                            Some(n.get_airplane_mode_quick_setting_button(opacity, icons))
                        }
                    }),
                ];

                quick_settings_section(buttons.into_iter().flatten().collect::<Vec<_>>(), opacity)
            }

            /// Menu of the standalone bluetooth module.
            pub fn bluetooth_menu(
                &self,
                id: Id,
                config: &ControlCenterModuleConfig,
                opacity: f32,
                icons: &IconTheme
            ) -> Element<'_, Message> {
                let button = self
                    .bluetooth
                    .as_ref()
                    .filter(|b| b.state != BluetoothState::Unavailable)
                    .and_then(|b| {
                        b.get_quick_setting_button(
                            id,
                            self.sub_menu,
                            config.bluetooth_more_cmd.is_some(),
                            opacity,
                            icons
                        )
                    });

                quick_settings_section(button.into_iter().collect::<Vec<_>>(), opacity)
            }

            /// Menu of the standalone power profile module, with the power
            /// actions underneath.
            pub fn power_profile_menu(
                &self,
                opacity: f32,
                config: &ControlCenterModuleConfig,
                icons: &IconTheme
            ) -> Element<'_, Message> {
                let profile = self
                    .upower
                    .as_ref()
                    .and_then(|u| u.power_profile.get_quick_setting_button(opacity, icons));

                Column::new()
                    .push(quick_settings_section(
                        profile.into_iter().collect::<Vec<_>>(),
                        opacity
                    ))
                    .push(power_menu(opacity, config, icons).map(Message::Power))
                    .width(Length::Fill)
                    .spacing(scale::scaled(16.0))
                    .into()
            }

            /// Quick toggle of the idle inhibitor, shared by the control center
            /// menu.
            #[allow(dead_code)]
            pub(super) fn idle_quick_button(
                &self,
                opacity: f32,
                icons: &IconTheme
            ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
                self.idle_inhibitor.as_ref().map(|inhibitor| {
                    (
                        quick_setting_button(
                            icons,
                            if inhibitor.is_inhibited() {
                                Icons::EyeOpened
                            } else {
                                Icons::EyeClosed
                            },
                            "Idle Inhibitor".to_string(),
                            None,
                            inhibitor.is_inhibited(),
                            Message::ToggleInhibitIdle,
                            None,
                            opacity
                        ),
                        None
                    )
                })
            }
        }
    }

    #[cfg(test)]
    mod tests {
        //! Unit tests for the settings menu layout helpers.

        use iced::{
            Element,
            widget::{button, text}
        };

        use super::{helpers::quick_settings_section, quick_setting_button};
        use crate::{
            components::icons::{IconTheme, Icons},
            modules::control_center::state::{Message, SubMenu}
        };

        #[test]
        fn quick_settings_section_pairs_buttons() {
            let button_a: Element<'_, Message> = button(text("a"))
                .on_press(Message::ToggleInhibitIdle)
                .into();
            let button_b: Element<'_, Message> = button(text("b"))
                .on_press(Message::ToggleInhibitIdle)
                .into();

            let section = quick_settings_section(vec![(button_a, None), (button_b, None)], 1.0);
            let children = section.as_widget().children();
            assert_eq!(children.len(), 1);
        }

        #[test]
        fn quick_settings_section_renders_menu_when_present() {
            let button_a: Element<'_, Message> = button(text("a"))
                .on_press(Message::ToggleInhibitIdle)
                .into();
            let menu: Element<'_, Message> = text("menu").into();

            let section = quick_settings_section(vec![(button_a, Some(menu))], 1.0);
            let children = section.as_widget().children();
            assert_eq!(children.len(), 2);
        }

        #[test]
        fn quick_setting_button_can_render_submenu_toggle() {
            let icons = IconTheme::default();
            let element: Element<'_, Message> = quick_setting_button(
                &icons,
                Icons::Power,
                "Test".into(),
                None,
                true,
                Message::ToggleInhibitIdle,
                Some((
                    SubMenu::Wifi,
                    Some(SubMenu::Wifi),
                    Message::ToggleInhibitIdle
                )),
                1.0
            );

            // A button renders a single row child that contains the submenu toggle.
            let children = element.as_widget().children();
            assert_eq!(children.len(), 1);
        }
    }

    pub use quick_button::quick_setting_button;

    pub trait ControlCenterViewExt {
        type ViewData<'a>;

        fn control_center_view<M>(
            &self,
            data: Self::ViewData<'_>
        ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
        where
            M: 'static + From<Message>;

        fn menu_view(
            &self,
            id: Id,
            config: &ControlCenterModuleConfig,
            opacity: f32,
            position: Position,
            icons: &IconTheme
        ) -> Element<'_, Message>;
    }

    impl ControlCenterViewExt for ControlCenter {
        type ViewData<'a> = &'a IconTheme;

        fn control_center_view<M>(
            &self,
            icons: Self::ViewData<'_>
        ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
        where
            M: 'static + From<Message>
        {
            self.render_bar(icons)
        }

        fn menu_view(
            &self,
            id: Id,
            config: &ControlCenterModuleConfig,
            opacity: f32,
            position: Position,
            icons: &IconTheme
        ) -> Element<'_, Message> {
            self.render_menu(id, config, opacity, position, icons)
        }
    }
}

pub mod audio {
    use iced::{
        Alignment, Element, Length, SurfaceId as Id, Theme,
        widget::{Column, Row, button, column, container, row, rule, slider}
    };

    use super::{Message, SubMenu};
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon},
            push_maybe::PushMaybe,
            scale,
            text::text
        },
        services::{
            ServiceEvent,
            audio::{AudioData, AudioService, DeviceType, Sinks}
        },
        style::{ghost_button_style, settings_button_style}
    };

    #[derive(Debug, Clone)]
    pub enum AudioMessage {
        Event(ServiceEvent<AudioService>),
        DefaultSinkChanged(String, String),
        DefaultSourceChanged(String, String),
        ToggleSinkMute,
        SinkVolumeChanged(i32),
        /// A wheel notch over the bar entry, `1` up and `-1` down.
        SinkVolumeWheel(i32),
        ToggleSourceMute,
        SourceVolumeChanged(i32),
        SinksMore(Id),
        SourcesMore(Id)
    }

    /// The wheel notch as a volume direction: `1` up, `-1` down.
    ///
    /// Stated once next to the message it feeds, so every place that takes the
    /// wheel — the bar entry, the open menu — reads the same direction.
    #[must_use]
    pub fn wheel_direction(delta: iced::mouse::ScrollDelta) -> i32 {
        use iced::mouse::ScrollDelta;

        let up = match delta {
            ScrollDelta::Lines {
                y, ..
            }
            | ScrollDelta::Pixels {
                y, ..
            } => y > 0.0
        };

        if up { 1 } else { -1 }
    }

    impl AudioData {
        pub fn sink_indicator<Message: 'static>(
            &self,
            icons: &IconTheme
        ) -> Option<Element<'static, Message>> {
            if !self.sinks.is_empty() {
                let icon_type = self.sinks.get_icon(&self.server_info.default_sink);

                Some(icon(icons, icon_type).into())
            } else {
                None
            }
        }

        pub fn audio_sliders(
            &self,
            sub_menu: Option<SubMenu>,
            opacity: f32,
            icons: &IconTheme
        ) -> (Option<Element<'_, Message>>, Option<Element<'_, Message>>) {
            let active_sink = self
                .sinks
                .iter()
                .find(|sink| sink.name == self.server_info.default_sink);

            let sink_slider = active_sink.map(|s| {
                audio_slider(
                    icons,
                    SliderType::Sink,
                    s.is_mute,
                    Message::Audio(AudioMessage::ToggleSinkMute),
                    self.cur_sink_volume,
                    |v| Message::Audio(AudioMessage::SinkVolumeChanged(v)),
                    if self.sinks.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                        Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sinks)))
                    } else {
                        None
                    },
                    opacity
                )
            });

            if self.sources.iter().any(|source| source.in_use) {
                let active_source = self
                    .sources
                    .iter()
                    .find(|source| source.name == self.server_info.default_source);

                let source_slider = active_source.map(|s| {
                    audio_slider(
                        icons,
                        SliderType::Source,
                        s.is_mute,
                        Message::Audio(AudioMessage::ToggleSourceMute),
                        self.cur_source_volume,
                        |v| Message::Audio(AudioMessage::SourceVolumeChanged(v)),
                        if self.sources.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                            Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sources)))
                        } else {
                            None
                        },
                        opacity
                    )
                });

                (sink_slider, source_slider)
            } else {
                (sink_slider, None)
            }
        }

        pub fn sinks_submenu(
            &self,
            id: Id,
            show_more: bool,
            opacity: f32,
            icons: &IconTheme
        ) -> Element<'_, Message> {
            audio_submenu(
                icons,
                self.sinks
                    .iter()
                    .flat_map(|s| {
                        s.ports.iter().map(|p| SubmenuEntry {
                            name:   format!("{}: {}", p.description, s.description),
                            device: p.device_type,
                            active: p.active && s.name == self.server_info.default_sink,
                            msg:    Message::Audio(AudioMessage::DefaultSinkChanged(
                                s.name.clone(),
                                p.name.clone()
                            ))
                        })
                    })
                    .collect(),
                if show_more {
                    Some(Message::Audio(AudioMessage::SinksMore(id)))
                } else {
                    None
                },
                opacity
            )
        }

        pub fn sources_submenu(
            &self,
            id: Id,
            show_more: bool,
            opacity: f32,
            icons: &IconTheme
        ) -> Element<'_, Message> {
            audio_submenu(
                icons,
                self.sources
                    .iter()
                    .flat_map(|s| {
                        s.ports.iter().map(|p| SubmenuEntry {
                            name:   format!("{}: {}", p.description, s.description),
                            device: p.device_type,
                            active: p.active && s.name == self.server_info.default_source,
                            msg:    Message::Audio(AudioMessage::DefaultSourceChanged(
                                s.name.clone(),
                                p.name.clone()
                            ))
                        })
                    })
                    .collect(),
                if show_more {
                    Some(Message::Audio(AudioMessage::SourcesMore(id)))
                } else {
                    None
                },
                opacity
            )
        }
    }

    pub enum SliderType {
        Sink,
        Source
    }

    pub fn audio_slider<'a, Message: 'a + Clone>(
        icons: &IconTheme,
        slider_type: SliderType,
        is_mute: bool,
        toggle_mute: Message,
        volume: i32,
        volume_changed: impl Fn(i32) -> Message + 'a,
        with_submenu: Option<(Option<SubMenu>, Message)>,
        opacity: f32
    ) -> Element<'a, Message> {
        Row::new()
            .push(
                button(icon(
                    icons,
                    if is_mute {
                        match slider_type {
                            SliderType::Sink => Icons::Speaker0,
                            SliderType::Source => Icons::Mic0
                        }
                    } else {
                        match slider_type {
                            SliderType::Sink => Icons::Speaker3,
                            SliderType::Source => Icons::Mic1
                        }
                    }
                ))
                .padding([
                    8,
                    match slider_type {
                        SliderType::Sink => 13,
                        SliderType::Source => 14
                    }
                ])
                .on_press(toggle_mute)
                .style(settings_button_style(opacity))
            )
            .push(
                slider(0..=100, volume, volume_changed)
                    .step(1)
                    .width(Length::Fill)
            )
            .push_maybe(with_submenu.map(|(submenu, msg)| {
                button(icon(
                    icons,
                    match (slider_type, submenu) {
                        (SliderType::Sink, Some(SubMenu::Sinks)) => Icons::Close,
                        (SliderType::Source, Some(SubMenu::Sources)) => Icons::Close,
                        _ => Icons::RightArrow
                    }
                ))
                .padding([scale::scaled(8.0), scale::scaled(13.0)])
                .on_press(msg)
                .style(settings_button_style(opacity))
            }))
            .align_y(Alignment::Center)
            .spacing(scale::scaled(8.0))
            .into()
    }

    pub struct SubmenuEntry<Message> {
        pub name:   String,
        pub device: DeviceType,
        pub active: bool,
        pub msg:    Message
    }

    pub fn audio_submenu<'a, Message: 'a + Clone>(
        icons: &IconTheme,
        entries: Vec<SubmenuEntry<Message>>,
        more_msg: Option<Message>,
        opacity: f32
    ) -> Element<'a, Message> {
        let entries = Column::with_children(
            entries
                .into_iter()
                .map(|e| {
                    if e.active {
                        container(
                            row!(icon(icons, e.device.get_icon()), text(e.name))
                                .align_y(Alignment::Center)
                                .spacing(scale::scaled(16.0))
                                .padding([scale::scaled(4.0), scale::scaled(12.0)])
                        )
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().success),
                            ..Default::default()
                        })
                        .into()
                    } else {
                        button(
                            row!(icon(icons, e.device.get_icon()), text(e.name))
                                .spacing(scale::scaled(16.0))
                                .align_y(Alignment::Center)
                        )
                        .on_press(e.msg)
                        .padding([scale::scaled(4.0), scale::scaled(12.0)])
                        .width(Length::Fill)
                        .style(ghost_button_style(opacity))
                        .into()
                    }
                })
                .collect::<Vec<_>>()
        )
        .spacing(scale::scaled(4.0))
        .into();

        match more_msg {
            Some(more_msg) => column!(
                entries,
                rule::horizontal(1),
                button("More")
                    .on_press(more_msg)
                    .padding([scale::scaled(4.0), scale::scaled(12.0)])
                    .width(Length::Fill)
                    .style(ghost_button_style(opacity)),
            )
            .spacing(scale::scaled(12.0))
            .into(),
            _ => entries
        }
    }
}
pub mod bluetooth {
    use iced::{
        Element, Length, SurfaceId as Id, Theme,
        widget::{Column, Row, button, column, container, row, rule}
    };

    use super::{Message, SubMenu, quick_setting_button};
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon},
            push_maybe::PushMaybe,
            scale,
            text::text
        },
        services::{
            ServiceEvent,
            bluetooth::{BluetoothData, BluetoothService, BluetoothState}
        },
        style::ghost_button_style
    };

    #[derive(Debug, Clone)]
    pub enum BluetoothMessage {
        Event(ServiceEvent<BluetoothService>),
        Toggle,
        ConnectDevice(zbus::zvariant::OwnedObjectPath),
        DisconnectDevice(zbus::zvariant::OwnedObjectPath),
        More(Id)
    }

    impl BluetoothData {
        pub fn get_quick_setting_button(
            &self,
            id: Id,
            sub_menu: Option<SubMenu>,
            show_more_button: bool,
            opacity: f32,
            icons: &IconTheme
        ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
            Some((
                quick_setting_button(
                    icons,
                    Icons::Bluetooth,
                    "Bluetooth".to_owned(),
                    None,
                    self.state == BluetoothState::Active,
                    Message::Bluetooth(BluetoothMessage::Toggle),
                    (self.state == BluetoothState::Active).then(|| {
                        (
                            SubMenu::Bluetooth,
                            sub_menu,
                            Message::ToggleSubMenu(SubMenu::Bluetooth)
                        )
                    }),
                    opacity
                ),
                sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Bluetooth)
                    .map(|_| self.bluetooth_menu(id, show_more_button, opacity, icons))
            ))
        }

        pub fn bluetooth_menu(
            &self,
            id: Id,
            show_more_button: bool,
            opacity: f32,
            icons: &IconTheme
        ) -> Element<'_, Message> {
            let main = if self.devices.is_empty() {
                container(text("No paired devices"))
                    .width(Length::Fill)
                    .into()
            } else {
                Column::with_children(
                    self.devices
                        .iter()
                        .map(|d| {
                            Row::new()
                                .push(text(d.name.to_string()).width(Length::Fill))
                                .push_maybe(
                                    d.battery.map(|battery| Self::battery_level(battery, icons))
                                )
                                .push(
                                    button(text(if d.connected {
                                        "Disconnect"
                                    } else {
                                        "Connect"
                                    }))
                                    .padding([scale::scaled(4.0), scale::scaled(12.0)])
                                    .style(ghost_button_style(opacity))
                                    .on_press(
                                        Message::Bluetooth(if d.connected {
                                            BluetoothMessage::DisconnectDevice(d.path.clone())
                                        } else {
                                            BluetoothMessage::ConnectDevice(d.path.clone())
                                        })
                                    )
                                )
                                .spacing(scale::scaled(8.0))
                                .align_y(iced::Alignment::Center)
                                .into()
                        })
                        .collect::<Vec<Element<'_, Message>>>()
                )
                .spacing(scale::scaled(8.0))
                .width(Length::Fill)
                .into()
            };

            if show_more_button {
                column!(
                    main,
                    rule::horizontal(1),
                    button("More")
                        .on_press(Message::Bluetooth(BluetoothMessage::More(id)))
                        .padding([scale::scaled(4.0), scale::scaled(12.0)])
                        .width(Length::Fill)
                        .style(ghost_button_style(opacity))
                )
                .spacing(scale::scaled(12.0))
                .into()
            } else {
                main
            }
        }

        fn battery_level<'a>(battery: u8, icons: &IconTheme) -> Element<'a, Message> {
            container(
                row!(
                    icon(
                        icons,
                        match battery {
                            0..=20 => Icons::Battery0,
                            21..=40 => Icons::Battery1,
                            41..=60 => Icons::Battery2,
                            61..=80 => Icons::Battery3,
                            _ => Icons::Battery4
                        }
                    ),
                    text(format!("{battery}%"))
                )
                .spacing(scale::scaled(8.0))
                .width(Length::Shrink)
            )
            .style(move |theme: &Theme| container::Style {
                text_color: Some(if battery <= 20 {
                    theme.palette().danger
                } else {
                    theme.palette().text
                }),
                ..container::Style::default()
            })
            .into()
        }
    }
}
pub mod brightness {
    use iced::{
        Alignment, Element, Length,
        widget::{container, row, slider}
    };

    use super::Message;
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon},
            scale
        },
        services::{
            ServiceEvent,
            brightness::{BrightnessData, BrightnessService}
        }
    };

    #[derive(Debug, Clone)]
    pub enum BrightnessMessage {
        Event(ServiceEvent<BrightnessService>),
        Change(u32)
    }

    impl BrightnessData {
        pub fn brightness_slider(&self, icons: &IconTheme) -> Element<'_, Message> {
            row!(
                container(icon(icons, Icons::Brightness))
                    .padding([scale::scaled(8.0), scale::scaled(11.0)]),
                slider(0..=100, self.current * 100 / self.max, |v| {
                    Message::Brightness(BrightnessMessage::Change(v * self.max / 100))
                })
                .step(1_u32)
                .width(Length::Fill),
            )
            .align_y(Alignment::Center)
            .spacing(scale::scaled(8.0))
            .into()
        }
    }
}
pub mod network {
    use iced::{
        Alignment, Element, Length, SurfaceId as Id, Theme,
        widget::{Column, button, column, container, row, rule, scrollable, toggler}
    };

    use super::{Message, SubMenu, quick_setting_button};
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon},
            scale,
            text::text
        },
        services::{
            ServiceEvent,
            network::{
                AccessPoint, ActiveConnectionInfo, ConnectivityState, KnownConnection,
                NetworkData, NetworkService, Vpn
            }
        },
        style::{ghost_button_style, settings_button_style},
        utils::IndicatorState
    };

    #[derive(Debug, Clone)]
    pub enum NetworkMessage {
        Event(ServiceEvent<NetworkService>),
        ToggleWiFi,
        ScanNearByWiFi,
        WiFiMore(Id),
        VpnMore(Id),
        SelectAccessPoint(AccessPoint),
        RequestWiFiPassword(Id, String),
        ToggleVpn(Vpn),
        ToggleAirplaneMode
    }

    static WIFI_SIGNAL_ICONS: [Icons; 6] = [
        Icons::Wifi0,
        Icons::Wifi1,
        Icons::Wifi2,
        Icons::Wifi3,
        Icons::Wifi4,
        Icons::Wifi5
    ];

    static WIFI_LOCK_SIGNAL_ICONS: [Icons; 5] = [
        Icons::WifiLock1,
        Icons::WifiLock2,
        Icons::WifiLock3,
        Icons::WifiLock4,
        Icons::WifiLock5
    ];

    impl ActiveConnectionInfo {
        /// Maps a signal strength to its icon bucket, whatever the backend
        /// sends.
        ///
        /// The strength is clamped first: a backend can report a value past one
        /// hundred — a wrapped negative RSSI does exactly that — and an index
        /// computed from it unclamped walked off the end of the icon tables.
        fn signal_bucket(signal: u8) -> usize {
            f32::round(f32::from(signal.min(100)) / 100. * 4.) as usize
        }

        pub fn get_wifi_icon(signal: u8) -> Icons {
            WIFI_SIGNAL_ICONS[1 + Self::signal_bucket(signal)]
        }

        pub fn get_wifi_lock_icon(signal: u8) -> Icons {
            WIFI_LOCK_SIGNAL_ICONS[Self::signal_bucket(signal)]
        }

        pub fn get_icon(&self) -> Icons {
            match self {
                Self::WiFi {
                    strength, ..
                } => Self::get_wifi_icon(*strength),
                Self::Wired {
                    ..
                } => Icons::Ethernet,
                Self::Vpn {
                    ..
                } => Icons::Vpn
            }
        }

        pub fn get_indicator_state(&self) -> IndicatorState {
            match self {
                Self::WiFi {
                    strength: 0 | 1, ..
                } => IndicatorState::Warning,
                _ => IndicatorState::Normal
            }
        }
    }

    impl super::ControlCenter {
        /// One-look summary of the connection, for the pointer resting on the
        /// network module.
        pub fn network_hint(&self) -> Option<String> {
            self.network
                .as_ref()
                .map(|service| service.connection_hint())
        }
    }

    impl NetworkData {
        /// States the connection the way its hover reads: the network, the
        /// signal, the frequency, the interface, the addressing — every fact
        /// the bar holds, one per line — or the one word explaining why
        /// there is nothing to state.
        pub fn connection_hint(&self) -> String {
            let mut lines = Vec::new();
            let mut vpns = Vec::new();

            for connection in &self.active_connections {
                match connection {
                    ActiveConnectionInfo::WiFi {
                        name,
                        strength,
                        ..
                    } => {
                        lines.push(format!("Network: {name}"));
                        lines.push(match self.link.signal_dbm {
                            Some(dbm) => format!("Signal strength: {dbm}dBm ({strength}%)"),
                            None => format!("Signal strength: {strength}%")
                        });

                        if let Some(mhz) = self.link.frequency_mhz {
                            lines.push(format!("Frequency: {mhz}MHz"));
                        }
                    }
                    ActiveConnectionInfo::Wired {
                        name,
                        speed
                    } => {
                        lines.push(format!("Wired: {name}"));

                        if *speed > 0 {
                            lines.push(format!("Speed: {speed} Mb/s"));
                        }
                    }
                    ActiveConnectionInfo::Vpn {
                        name, ..
                    } => vpns.push(name.clone())
                }
            }

            if !lines.is_empty() {
                if let Some(interface) = &self.link.interface {
                    lines.push(format!("Interface: {interface}"));
                }

                if let Some(address) = &self.link.address {
                    lines.push(format!("IP: {address}"));
                }

                if let Some(gateway) = &self.link.gateway {
                    lines.push(format!("Gateway: {gateway}"));
                }

                if let Some(netmask) = &self.link.netmask {
                    lines.push(format!("Netmask: {netmask}"));
                }
            }

            for vpn in vpns {
                lines.push(format!("VPN: {vpn}"));
            }

            if lines.is_empty() {
                lines.push(
                    if self.airplane_mode {
                        "Airplane mode"
                    } else if self.wifi_present && !self.wifi_enabled {
                        "Wi-Fi off"
                    } else {
                        "Disconnected"
                    }
                    .to_owned()
                );
            }

            lines.join("\n")
        }

        pub fn get_connection_indicator<Message: 'static>(
            &self,
            icons: &IconTheme
        ) -> Option<Element<'static, Message>> {
            if self.airplane_mode || !self.wifi_present {
                None
            } else {
                Some(
                    self.active_connections
                        .iter()
                        .find(|c| {
                            matches!(c, ActiveConnectionInfo::WiFi { .. })
                                || matches!(c, ActiveConnectionInfo::Wired { .. })
                        })
                        .map_or_else(
                            || icon(icons, Icons::Wifi0).into(),
                            |a| {
                                let icon_type = a.get_icon();
                                let state = (self.connectivity, a.get_indicator_state());

                                container(icon(icons, icon_type))
                                    .style(move |theme: &Theme| container::Style {
                                        text_color: match state {
                                            (ConnectivityState::Full, IndicatorState::Warning) => {
                                                Some(theme.palette().warning)
                                            }
                                            (ConnectivityState::Full, _) => None,
                                            _ => Some(theme.palette().danger)
                                        },
                                        ..Default::default()
                                    })
                                    .into()
                            }
                        )
                )
            }
        }

        pub fn get_vpn_indicator<Message: 'static>(
            &self,
            icons: &IconTheme
        ) -> Option<Element<'static, Message>> {
            self.active_connections
                .iter()
                .find(|c| matches!(c, ActiveConnectionInfo::Vpn { .. }))
                .map(|a| {
                    let icon_type = a.get_icon();

                    container(icon(icons, icon_type))
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.extended_palette().danger.weak.color),
                            ..Default::default()
                        })
                        .into()
                })
        }

        pub fn get_wifi_quick_setting_button(
            &self,
            id: Id,
            sub_menu: Option<SubMenu>,
            show_more_button: bool,
            opacity: f32,
            icons: &IconTheme
        ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
            if self.wifi_present {
                let active_connection = self.active_connections.iter().find_map(|c| match c {
                    ActiveConnectionInfo::WiFi {
                        name,
                        strength,
                        ..
                    } => Some((name, strength, c.get_icon())),
                    _ => None
                });

                Some((
                    quick_setting_button(
                        icons,
                        active_connection.map_or_else(|| Icons::Wifi0, |(_, _, icon)| icon),
                        "Wi-Fi".to_string(),
                        active_connection
                            .map(|(name, strength, _)| format!("{name} ({}%)", strength,)),
                        self.wifi_enabled,
                        Message::Network(NetworkMessage::ToggleWiFi),
                        self.wifi_enabled.then(|| {
                            (
                                SubMenu::Wifi,
                                sub_menu,
                                Message::ToggleSubMenu(SubMenu::Wifi)
                            )
                        }),
                        opacity
                    ),
                    sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Wifi)
                        .map(|_| {
                            self.wifi_menu(
                                id,
                                active_connection
                                    .map(|(name, strengh, _)| (name.as_str(), *strengh)),
                                show_more_button,
                                opacity,
                                icons
                            )
                            .map(Message::Network)
                        })
                ))
            } else {
                None
            }
        }

        pub fn get_vpn_quick_setting_button(
            &self,
            id: Id,
            sub_menu: Option<SubMenu>,
            show_more_button: bool,
            opacity: f32,
            icons: &IconTheme
        ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
            self.known_connections
                .iter()
                .any(|c| matches!(c, KnownConnection::Vpn { .. }))
                .then(|| {
                    (
                        quick_setting_button(
                            icons,
                            Icons::Vpn,
                            "Vpn".to_string(),
                            None,
                            self.active_connections
                                .iter()
                                .any(|c| matches!(c, ActiveConnectionInfo::Vpn { .. })),
                            Message::ToggleSubMenu(SubMenu::Vpn),
                            None,
                            opacity
                        ),
                        sub_menu
                            .filter(|menu_type| *menu_type == SubMenu::Vpn)
                            .map(|_| {
                                self.vpn_menu(id, show_more_button, opacity)
                                    .map(Message::Network)
                            })
                    )
                })
        }

        pub fn wifi_menu(
            &self,
            id: Id,
            active_connection: Option<(&str, u8)>,
            show_more_button: bool,
            opacity: f32,
            icons: &IconTheme
        ) -> Element<'_, NetworkMessage> {
            let main = column!(
            row!(
                text("Nearby Wifi").width(Length::Fill),
                text(if self.scanning_nearby_wifi {
                    "Scanning..."
                } else {
                    ""
                })
                .size(scale::scaled(12.0)),
                button(icon(icons, Icons::Refresh))
                    .padding([scale::scaled(4.0), scale::scaled(10.0)])
                    .style(settings_button_style(opacity))
                    .on_press(NetworkMessage::ScanNearByWiFi),
            )
            .spacing(scale::scaled(8.0))
            .width(Length::Fill)
            .align_y(Alignment::Center),
            rule::horizontal(1),
            container(scrollable(
                Column::with_children(
                    self.wireless_access_points
                    .iter()
                    .filter_map(|ac| if active_connection.is_some_and(|(ssid, _)| ssid == ac.ssid) {Some((ac, true))} else {None })
                    .chain(self.wireless_access_points
                        .iter()
                        .filter_map(|ac| if active_connection.is_some_and(|(ssid, _)| ssid == ac.ssid) {None} else {Some((ac, false))})
                    )
                        .map(|(ac, is_active)| {
                            let is_known = self.known_connections.iter().any(|c| {
                                matches!(
                                    c,
                                    KnownConnection::AccessPoint(AccessPoint { ssid, .. }) if ssid == &ac.ssid
                                )
                            });

                            button(
                                container(
                                    row!(
                                        icon(icons, if ac.public {
                                            ActiveConnectionInfo::get_wifi_icon(ac.strength)
                                        } else {
                                            ActiveConnectionInfo::get_wifi_lock_icon(ac.strength)
                                        })
                                        .width(Length::Shrink),
                                        text(ac.ssid.clone()).width(Length::Fill),
                                        text(format!("{}%", ac.strength)).size(scale::scaled(12.0)),
                                    )
                                    .align_y(Alignment::Center)
                                    .spacing(scale::scaled(8.0)),
                                )
                                .style(move |theme: &Theme| {
                                    container::Style {
                                        text_color: if is_active {
                                            Some(theme.palette().success)
                                        } else {
                                            None
                                        },
                                        ..Default::default()
                                    }
                                }),
                            )
                            .style(ghost_button_style(opacity))
                            .padding([scale::scaled(8.0), scale::scaled(8.0)])
                            .on_press_maybe(if !is_active {
                                Some(if is_known {
                                    NetworkMessage::SelectAccessPoint(ac.clone())
                                } else {
                                    NetworkMessage::RequestWiFiPassword(id, ac.ssid.clone())
                                })
                            } else {
                                None
                            })
                            .width(Length::Fill)
                            .into()
                        })
                        .collect::<Vec<Element<NetworkMessage>>>(),
                )
                .spacing(scale::scaled(4.0))
            ))
            .max_height(200),
        )
        .width(Length::Fill)
        .spacing(scale::scaled(8.0));

            if show_more_button {
                column!(
                    main,
                    rule::horizontal(1),
                    button("More")
                        .on_press(NetworkMessage::WiFiMore(id))
                        .padding([scale::scaled(4.0), scale::scaled(12.0)])
                        .width(Length::Fill)
                        .style(ghost_button_style(opacity))
                )
                .spacing(scale::scaled(12.0))
                .into()
            } else {
                main.into()
            }
        }

        pub fn vpn_menu(
            &self,
            id: Id,
            show_more_button: bool,
            opacity: f32
        ) -> Element<'_, NetworkMessage> {
            let main = Column::with_children(
            self.known_connections
                .iter()
                .filter_map(|c| match c {
                    KnownConnection::Vpn(vpn) => Some(vpn),
                    _ => None,
                })
                .map(|vpn| {
                    let is_active = self.active_connections.iter().any(
                        |c| matches!(c, ActiveConnectionInfo::Vpn { name, .. } if name == &vpn.name),
                    );

                    row!(
                        text(vpn.name.to_string()).width(Length::Fill),
                        toggler(is_active)
                            .on_toggle(|_| { NetworkMessage::ToggleVpn(vpn.clone()) })
                            .width(Length::Shrink),
                    )
                    .into()
                })
                .collect::<Vec<Element<NetworkMessage>>>(),
        )
        .width(Length::Fill)
        .spacing(scale::scaled(8.0));

            if show_more_button {
                column!(
                    main,
                    rule::horizontal(1),
                    button("More")
                        .on_press(NetworkMessage::VpnMore(id))
                        .padding([scale::scaled(4.0), scale::scaled(12.0)])
                        .width(Length::Fill)
                        .style(ghost_button_style(opacity))
                )
                .spacing(scale::scaled(12.0))
                .into()
            } else {
                main.into()
            }
        }

        pub fn get_airplane_mode_quick_setting_button(
            &self,
            opacity: f32,
            icons: &IconTheme
        ) -> (Element<'_, Message>, Option<Element<'_, Message>>) {
            (
                quick_setting_button(
                    icons,
                    Icons::Airplane,
                    "Airplane Mode".to_string(),
                    None,
                    self.airplane_mode,
                    Message::Network(NetworkMessage::ToggleAirplaneMode),
                    None,
                    opacity
                ),
                None
            )
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn every_possible_signal_yields_a_wifi_icon_without_panicking() {
            for signal in u8::MIN..=u8::MAX {
                let _ = ActiveConnectionInfo::get_wifi_icon(signal);
            }
        }

        #[test]
        fn every_possible_signal_yields_a_wifi_lock_icon_without_panicking() {
            for signal in u8::MIN..=u8::MAX {
                let _ = ActiveConnectionInfo::get_wifi_lock_icon(signal);
            }
        }

        #[test]
        fn signal_quartiles_pick_ascending_wifi_icons() {
            assert_eq!(ActiveConnectionInfo::get_wifi_icon(0), Icons::Wifi1);
            assert_eq!(ActiveConnectionInfo::get_wifi_icon(25), Icons::Wifi2);
            assert_eq!(ActiveConnectionInfo::get_wifi_icon(50), Icons::Wifi3);
            assert_eq!(ActiveConnectionInfo::get_wifi_icon(75), Icons::Wifi4);
            assert_eq!(ActiveConnectionInfo::get_wifi_icon(100), Icons::Wifi5);
        }

        #[test]
        fn signal_quartiles_pick_ascending_wifi_lock_icons() {
            assert_eq!(
                ActiveConnectionInfo::get_wifi_lock_icon(0),
                Icons::WifiLock1
            );
            assert_eq!(
                ActiveConnectionInfo::get_wifi_lock_icon(25),
                Icons::WifiLock2
            );
            assert_eq!(
                ActiveConnectionInfo::get_wifi_lock_icon(50),
                Icons::WifiLock3
            );
            assert_eq!(
                ActiveConnectionInfo::get_wifi_lock_icon(75),
                Icons::WifiLock4
            );
            assert_eq!(
                ActiveConnectionInfo::get_wifi_lock_icon(100),
                Icons::WifiLock5
            );
        }

        #[test]
        fn a_signal_past_one_hundred_stays_in_the_top_bucket() {
            assert_eq!(ActiveConnectionInfo::get_wifi_icon(u8::MAX), Icons::Wifi5);
            assert_eq!(
                ActiveConnectionInfo::get_wifi_lock_icon(u8::MAX),
                Icons::WifiLock5
            );
        }

        #[test]
        fn the_hover_states_the_wifi_and_its_strength() {
            let data = NetworkData {
                active_connections: vec![ActiveConnectionInfo::WiFi {
                    id:       "home".to_owned(),
                    name:     "HomeNet".to_owned(),
                    strength: 87
                }],
                ..NetworkData::default()
            };

            assert_eq!(
                data.connection_hint(),
                "Network: HomeNet\nSignal strength: 87%"
            );
        }

        #[test]
        fn the_hover_states_every_fact_of_the_link_it_holds() {
            let data = NetworkData {
                active_connections: vec![ActiveConnectionInfo::WiFi {
                    id:       "home".to_owned(),
                    name:     "HomeNet".to_owned(),
                    strength: 87
                }],
                link: crate::services::network::LinkDetails {
                    interface:     Some("wlan0".to_owned()),
                    signal_dbm:    Some(-27),
                    frequency_mhz: Some(5320),
                    address:       Some("192.168.2.19/24".to_owned()),
                    gateway:       Some("192.168.2.253".to_owned()),
                    netmask:       Some("255.255.255.0".to_owned())
                },
                ..NetworkData::default()
            };

            assert_eq!(
                data.connection_hint(),
                "Network: HomeNet\nSignal strength: -27dBm (87%)\nFrequency: \
             5320MHz\nInterface: wlan0\nIP: 192.168.2.19/24\nGateway: \
             192.168.2.253\nNetmask: 255.255.255.0"
            );
        }

        #[test]
        fn the_hover_states_the_wire_and_any_vpn_on_top() {
            let data = NetworkData {
                active_connections: vec![
                    ActiveConnectionInfo::Wired {
                        name:  "eth0".to_owned(),
                        speed: 1000
                    },
                    ActiveConnectionInfo::Vpn {
                        name:        "work".to_owned(),
                        object_path: zbus::zvariant::OwnedObjectPath::try_from("/").expect("path")
                    },
                ],
                ..NetworkData::default()
            };

            assert_eq!(
                data.connection_hint(),
                "Wired: eth0\nSpeed: 1000 Mb/s\nVPN: work"
            );
        }

        #[test]
        fn a_wire_without_a_reported_speed_keeps_that_line_to_itself() {
            let data = NetworkData {
                active_connections: vec![ActiveConnectionInfo::Wired {
                    name:  "eth0".to_owned(),
                    speed: 0
                }],
                ..NetworkData::default()
            };

            assert_eq!(data.connection_hint(), "Wired: eth0");
        }

        #[test]
        fn nothing_connected_names_the_reason() {
            assert_eq!(NetworkData::default().connection_hint(), "Disconnected");

            let airplane = NetworkData {
                airplane_mode: true,
                ..NetworkData::default()
            };
            assert_eq!(airplane.connection_hint(), "Airplane mode");

            let radio_off = NetworkData {
                wifi_present: true,
                wifi_enabled: false,
                ..NetworkData::default()
            };
            assert_eq!(radio_off.connection_hint(), "Wi-Fi off");
        }
    }
}
mod power {
    use iced::{
        Element, Length,
        widget::{button, column, row, rule}
    };

    use crate::{
        components::{
            icons::{IconTheme, Icons, icon},
            scale,
            text::text
        },
        config::ControlCenterModuleConfig,
        style::ghost_button_style,
        utils
    };

    #[derive(Debug, Clone)]
    pub enum PowerMessage {
        Suspend(String),
        Reboot(String),
        Shutdown(String),
        Logout(String)
    }

    impl PowerMessage {
        pub fn update(self) {
            match self {
                PowerMessage::Suspend(cmd) => {
                    utils::launcher::suspend(cmd);
                }
                PowerMessage::Reboot(cmd) => {
                    utils::launcher::reboot(cmd);
                }
                PowerMessage::Shutdown(cmd) => {
                    utils::launcher::shutdown(cmd);
                }
                PowerMessage::Logout(cmd) => {
                    utils::launcher::logout(cmd);
                }
            }
        }
    }

    pub fn power_menu<'a>(
        opacity: f32,
        config: &ControlCenterModuleConfig,
        icons: &IconTheme
    ) -> Element<'a, PowerMessage> {
        column!(
            button(
                row!(icon(icons, Icons::Suspend), text("Suspend")).spacing(scale::scaled(16.0))
            )
            .padding([scale::scaled(4.0), scale::scaled(12.0)])
            .on_press(PowerMessage::Suspend(config.suspend_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
            button(row!(icon(icons, Icons::Reboot), text("Reboot")).spacing(scale::scaled(16.0)))
                .padding([scale::scaled(4.0), scale::scaled(12.0)])
                .on_press(PowerMessage::Reboot(config.reboot_cmd.clone()))
                .width(Length::Fill)
                .style(ghost_button_style(opacity)),
            button(row!(icon(icons, Icons::Power), text("Shutdown")).spacing(scale::scaled(16.0)))
                .padding([scale::scaled(4.0), scale::scaled(12.0)])
                .on_press(PowerMessage::Shutdown(config.shutdown_cmd.clone()))
                .width(Length::Fill)
                .style(ghost_button_style(opacity)),
            rule::horizontal(1),
            button(row!(icon(icons, Icons::Logout), text("Logout")).spacing(scale::scaled(16.0)))
                .padding([scale::scaled(4.0), scale::scaled(12.0)])
                .on_press(PowerMessage::Logout(config.logout_cmd.clone()))
                .width(Length::Fill)
                .style(ghost_button_style(opacity)),
        )
        .padding(scale::scaled(8.0))
        .width(Length::Fill)
        .spacing(scale::scaled(8.0))
        .into()
    }
}
mod upower {
    use iced::{
        Alignment, Element, Theme,
        widget::{Container, container, row}
    };

    use super::{Message, quick_setting_button};
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon},
            scale,
            text::text
        },
        services::{
            ServiceEvent,
            upower::{BatteryData, BatteryStatus, PowerProfile, UPowerService}
        },
        utils::{IndicatorState, format_duration}
    };

    #[derive(Clone, Debug)]
    pub enum UPowerMessage {
        Event(ServiceEvent<UPowerService>),
        TogglePowerProfile
    }

    impl BatteryData {
        pub fn indicator<Message: 'static>(&self, icons: &IconTheme) -> Element<'static, Message> {
            let icon_type = self.get_icon();
            let state = self.get_indicator_state();

            container(
                row!(icon(icons, icon_type), text(format!("{}%", self.capacity)))
                    .spacing(scale::icon_gap())
                    .align_y(Alignment::Center)
            )
            .style(move |theme: &Theme| container::Style {
                text_color: Some(match state {
                    IndicatorState::Success => theme.palette().success,
                    IndicatorState::Danger => theme.palette().danger,
                    _ => theme.palette().text
                }),
                ..Default::default()
            })
            .into()
        }

        pub fn settings_indicator<'a, Message: 'static>(
            &self,
            icons: &IconTheme
        ) -> Container<'a, Message> {
            let state = self.get_indicator_state();

            container({
                let battery_info = container(
                    row!(
                        icon(icons, self.get_icon()),
                        text(format!("{}%", self.capacity))
                    )
                    .spacing(scale::icon_gap())
                )
                .style(move |theme: &Theme| container::Style {
                    text_color: Some(match state {
                        IndicatorState::Success => theme.palette().success,
                        IndicatorState::Danger => theme.palette().danger,
                        _ => theme.palette().text
                    }),
                    ..Default::default()
                });

                match self.status {
                    BatteryStatus::Charging(remaining) if self.capacity < 95 => row!(
                        battery_info,
                        text(format!("Full in {}", format_duration(&remaining)))
                    )
                    .spacing(scale::scaled(16.0)),
                    BatteryStatus::Discharging(remaining) if self.capacity < 95 => row!(
                        battery_info,
                        text(format!("Empty in {}", format_duration(&remaining)))
                    )
                    .spacing(scale::scaled(16.0)),
                    _ => row!(battery_info)
                }
            })
            .padding([scale::scaled(8.0), scale::scaled(4.0)])
        }
    }

    impl PowerProfile {
        pub fn indicator<Message: 'static>(
            &self,
            icons: &IconTheme
        ) -> Option<Element<'static, Message>> {
            match self {
                PowerProfile::Balanced => None,
                PowerProfile::Performance => Some(
                    container(icon(icons, Icons::Performance))
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().danger),
                            ..Default::default()
                        })
                        .into()
                ),
                PowerProfile::PowerSaver => Some(
                    container(icon(icons, Icons::PowerSaver))
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().success),
                            ..Default::default()
                        })
                        .into()
                ),
                PowerProfile::Unknown => None
            }
        }

        pub fn get_quick_setting_button(
            &self,
            opacity: f32,
            icons: &IconTheme
        ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
            if !matches!(self, PowerProfile::Unknown) {
                Some((
                    quick_setting_button(
                        icons,
                        (*self).into(),
                        match self {
                            PowerProfile::Balanced => "Balanced",
                            PowerProfile::Performance => "Performance",
                            PowerProfile::PowerSaver => "Power Saver",
                            PowerProfile::Unknown => ""
                        }
                        .to_string(),
                        None,
                        true,
                        Message::UPower(UPowerMessage::TogglePowerProfile),
                        None,
                        opacity
                    ),
                    None
                ))
            } else {
                None
            }
        }
    }
}

pub use audio::AudioMessage;
pub use bluetooth::BluetoothMessage;
pub use brightness::BrightnessMessage;
pub use network::NetworkMessage;
pub use power::PowerMessage;
pub use state::{ControlCenter, Message, SubMenu};
pub use upower::UPowerMessage;
pub use view::{ControlCenterViewExt, quick_setting_button};
