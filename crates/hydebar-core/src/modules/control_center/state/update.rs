//! Handling of settings menu messages.

use log::info;
use tokio::runtime::Handle;

use super::super::{
    ControlCenter, Message, SubMenu, audio::AudioMessage, bluetooth::BluetoothMessage,
    brightness::BrightnessMessage, commands::ControlCenterCommandExt, network::NetworkMessage,
    upower::UPowerMessage
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
        self.runtime.clone()
    }

    pub(crate) fn sender(&self) -> Option<ModuleEventSender<Message>> {
        self.sender.clone()
    }

    /// Schedules the release of an activation that outlives `delay`.
    ///
    /// Without a delay the inhibitor stays until it is toggled off,
    /// which is what a configuration naming no timeout asks
    /// for.
    fn arm_idle_release(&mut self, delay: Option<std::time::Duration>) {
        let (Some(delay), Some(runtime), Some(sender)) = (delay, self.runtime(), self.sender())
        else {
            return;
        };

        self.idle_release = Some(runtime.spawn(async move {
            tokio::time::sleep(delay).await;

            sender.send(Message::ReleaseInhibitIdle);
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
            Message::Audio(msg) => {
                self.handle_audio(msg, config, outputs, main_config);
            }
            Message::UPower(msg) => {
                self.handle_upower(msg);
            }
            Message::Network(msg) => {
                self.handle_network(msg, config, outputs, main_config);
            }
            Message::Bluetooth(msg) => {
                self.handle_bluetooth(msg, config, outputs, main_config);
            }
            Message::Brightness(msg) => {
                self.handle_brightness(msg);
            }
            Message::ToggleSubMenu(menu_type) => {
                if self.sub_menu == Some(menu_type) {
                    self.sub_menu.take();
                } else {
                    self.sub_menu.replace(menu_type);

                    if menu_type == SubMenu::Wifi {
                        let _spawned = self.spawn_network_command(NetworkCommand::ScanNearByWiFi);
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
                    crate::utils::launcher::execute_command(lock_cmd.clone());
                }
            }
            Message::Power(msg) => {
                msg.update();
            }
            Message::PasswordDialog(msg) => {
                self.handle_password_dialog(msg, outputs, main_config);
            }
        }
    }

    fn handle_audio(
        &mut self,
        msg: AudioMessage,
        config: &ControlCenterModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config
    ) {
        match msg {
            AudioMessage::Event(event) => match *event {
                ServiceEvent::Init(service) => {
                    self.audio = Some(service);
                }
                ServiceEvent::Update(data) => {
                    if let Some(audio) = self.audio.as_mut() {
                        audio.update(data);

                        if self.sub_menu == Some(SubMenu::Sinks) && audio.sinks.len() < 2 {
                            self.sub_menu = None;
                        }

                        if self.sub_menu == Some(SubMenu::Sources) && audio.sources.len() < 2 {
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
                let _spawned = self.spawn_audio_command(AudioCommand::SinkVolume(value));
            }
            AudioMessage::SinkVolumeWheel(direction) => {
                if let Some(audio) = self.audio.as_ref() {
                    let value =
                        (audio.cur_sink_volume + direction * WHEEL_VOLUME_STEP).clamp(0, 100);

                    let _spawned = self.spawn_audio_command(AudioCommand::SinkVolume(value));
                }
            }
            AudioMessage::DefaultSinkChanged(name, port) => {
                let _spawned = self.spawn_audio_command(AudioCommand::DefaultSink(name, port));
            }
            AudioMessage::ToggleSourceMute => {
                let _spawned = self.spawn_audio_command(AudioCommand::ToggleSourceMute);
            }
            AudioMessage::SourceVolumeChanged(value) => {
                let _spawned = self.spawn_audio_command(AudioCommand::SourceVolume(value));
            }
            AudioMessage::DefaultSourceChanged(name, port) => {
                let _spawned = self.spawn_audio_command(AudioCommand::DefaultSource(name, port));
            }
            AudioMessage::SinksMore(id) => {
                if let Some(cmd) = &config.audio_sinks_more_cmd {
                    crate::utils::launcher::execute_command(cmd.clone());
                    let _ = outputs.close_menu::<Message>(id, main_config);
                }
            }
            AudioMessage::SourcesMore(id) => {
                if let Some(cmd) = &config.audio_sources_more_cmd {
                    crate::utils::launcher::execute_command(cmd.clone());
                    let _ = outputs.close_menu::<Message>(id, main_config);
                }
            }
        }
    }

    fn handle_upower(&mut self, msg: UPowerMessage) {
        match msg {
            UPowerMessage::Event(event) => match *event {
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
        }
    }

    fn handle_network(
        &mut self,
        msg: NetworkMessage,
        config: &ControlCenterModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config
    ) {
        match msg {
            NetworkMessage::Event(event) => match *event {
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

                let _spawned = self.spawn_network_command(NetworkCommand::ToggleAirplaneMode);
            }
            NetworkMessage::ToggleWiFi => {
                if self.sub_menu == Some(SubMenu::Wifi) {
                    self.sub_menu = None;
                }

                let _spawned = self.spawn_network_command(NetworkCommand::ToggleWiFi);
            }
            NetworkMessage::SelectAccessPoint(ac) => {
                let _spawned =
                    self.spawn_network_command(NetworkCommand::SelectAccessPoint((ac, None)));
            }
            NetworkMessage::RequestWiFiPassword(id, ssid) => {
                info!("Requesting password for {ssid}");
                self.password_dialog = Some((ssid, String::new()));
                let _ = outputs.request_keyboard::<Message>(id, main_config.menu_keyboard_focus);
            }
            NetworkMessage::ScanNearByWiFi => {
                let _spawned = self.spawn_network_command(NetworkCommand::ScanNearByWiFi);
            }
            NetworkMessage::WiFiMore(id) => {
                if let Some(cmd) = &config.wifi_more_cmd {
                    crate::utils::launcher::execute_command(cmd.clone());
                    let _ = outputs.close_menu::<Message>(id, main_config);
                }
            }
            NetworkMessage::VpnMore(id) => {
                if let Some(cmd) = &config.vpn_more_cmd {
                    crate::utils::launcher::execute_command(cmd.clone());
                    let _ = outputs.close_menu::<Message>(id, main_config);
                }
            }
            NetworkMessage::ToggleVpn(vpn) => {
                let _spawned = self.spawn_network_command(NetworkCommand::ToggleVpn(vpn));
            }
        }
    }

    fn handle_bluetooth(
        &mut self,
        msg: BluetoothMessage,
        config: &ControlCenterModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config
    ) {
        match msg {
            BluetoothMessage::Event(event) => match *event {
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

                    let _spawned = self.spawn_bluetooth_command(BluetoothCommand::Toggle);
                }
                None => {
                    log::warn!("Bluetooth service not initialized");
                }
            },
            BluetoothMessage::ConnectDevice(device_path) => {
                let _spawned =
                    self.spawn_bluetooth_command(BluetoothCommand::ConnectDevice(device_path));
            }
            BluetoothMessage::DisconnectDevice(device_path) => {
                let _spawned =
                    self.spawn_bluetooth_command(BluetoothCommand::DisconnectDevice(device_path));
            }
            BluetoothMessage::More(id) => {
                if let Some(cmd) = &config.bluetooth_more_cmd {
                    crate::utils::launcher::execute_command(cmd.clone());
                    let _ = outputs.close_menu::<Message>(id, main_config);
                }
            }
        }
    }

    fn handle_brightness(&mut self, msg: BrightnessMessage) {
        match msg {
            BrightnessMessage::Event(event) => match *event {
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
                let _spawned = self.spawn_brightness_command(BrightnessCommand::Set(value));
            }
        }
    }

    fn handle_password_dialog(
        &mut self,
        msg: password_dialog::Message,
        outputs: &Outputs,
        main_config: &crate::config::Config
    ) {
        match msg {
            password_dialog::Message::PasswordChanged(password) => {
                if let Some((_, current_password)) = &mut self.password_dialog {
                    *current_password = password;
                }
            }
            password_dialog::Message::DialogConfirmed(id) => {
                if let Some((ssid, password)) = self.password_dialog.take()
                    && let Some(network) = self.network.as_ref()
                    && let Some(access_point) = network
                        .wireless_access_points
                        .iter()
                        .find(|ap| ap.ssid == ssid)
                        .cloned()
                {
                    self.spawn_network_command(NetworkCommand::SelectAccessPoint((
                        access_point,
                        Some(password)
                    )));
                }

                let _ = outputs.release_keyboard::<Message>(id, main_config.menu_keyboard_focus);
            }
            password_dialog::Message::DialogCancelled(id) => {
                self.password_dialog = None;

                let _ = outputs.release_keyboard::<Message>(id, main_config.menu_keyboard_focus);
            }
        }
    }
}
