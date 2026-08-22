//! Handling of network messages: service events, radio toggles and
//! connection picks.

use iced::Task;
use log::info;

use super::super::super::{
    ControlCenter, Message, SubMenu, commands::ControlCenterCommandExt, network::NetworkMessage
};
use crate::{
    config::ControlCenterModuleConfig,
    outputs::Outputs,
    services::{
        ReadOnlyService, ServiceEvent,
        network::{NetworkCommand, NetworkEvent}
    }
};

impl ControlCenter {
    #[must_use = "the shell work a menu asks for does not happen unless the task is run"]
    pub(super) fn handle_network(
        &mut self,
        msg: NetworkMessage,
        config: &ControlCenterModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config
    ) -> Task<Message> {
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

                return outputs.request_keyboard::<Message>(id, main_config.menu_keyboard_focus);
            }
            NetworkMessage::ScanNearByWiFi => {
                let _spawned = self.spawn_network_command(NetworkCommand::ScanNearByWiFi);
            }
            NetworkMessage::WiFiMore(id) => {
                if let Some(cmd) = &config.wifi_more_cmd {
                    crate::utils::launcher::execute_command(cmd.clone());

                    return outputs.close_menu::<Message>(id, main_config);
                }
            }
            NetworkMessage::VpnMore(id) => {
                if let Some(cmd) = &config.vpn_more_cmd {
                    crate::utils::launcher::execute_command(cmd.clone());

                    return outputs.close_menu::<Message>(id, main_config);
                }
            }
            NetworkMessage::ToggleVpn(vpn) => {
                let _spawned = self.spawn_network_command(NetworkCommand::ToggleVpn(vpn));
            }
        }

        Task::none()
    }
}
