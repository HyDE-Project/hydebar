//! Handling of bluetooth messages: service events, the radio toggle and
//! device connections.

use super::super::super::{
    ControlCenter, Message, SubMenu, bluetooth::BluetoothMessage, commands::ControlCenterCommandExt
};
use crate::{
    config::ControlCenterModuleConfig,
    outputs::Outputs,
    services::{ReadOnlyService, ServiceEvent, bluetooth::BluetoothCommand}
};

impl ControlCenter {
    pub(super) fn handle_bluetooth(
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
}
