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
