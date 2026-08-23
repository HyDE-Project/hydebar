//! Messages and submenu identifiers of the settings module.

use super::super::{
    audio::AudioMessage, bluetooth::BluetoothMessage, brightness::BrightnessMessage,
    network::NetworkMessage, power::PowerMessage, upower::UPowerMessage
};
use crate::password_dialog;

/// Everything the quick settings answer to.
#[derive(Debug, Clone)]
pub enum Message {
    /// A press asked for the panel, from this surface and this button.
    ToggleMenu(iced::SurfaceId, crate::position_button::ButtonUIRef),
    /// Something happened in the battery or the power profile.
    UPower(UPowerMessage),
    /// Something happened in the links or the networks.
    Network(NetworkMessage),
    /// Something happened in the adapter or its devices.
    Bluetooth(BluetoothMessage),
    /// Something happened in the outputs or the inputs.
    Audio(AudioMessage),
    /// Something happened to the backlight.
    Brightness(BrightnessMessage),
    /// Turn keeping the screen awake on or off.
    ToggleInhibitIdle,
    /// Releases the inhibitor the configured timeout has outlived.
    ReleaseInhibitIdle,
    /// Lock the session.
    Lock,
    /// Shut down, restart, suspend or log out.
    Power(PowerMessage),
    /// Open or close one of the panel's sections.
    ToggleSubMenu(SubMenu),
    /// Something happened in the dialog asking for a secret.
    PasswordDialog(password_dialog::Message)
}

/// The sections the quick settings panel can open.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubMenu {
    /// The buttons that end the session or the machine.
    Power,
    /// The outputs sound can play to.
    Sinks,
    /// The inputs sound can be recorded from.
    Sources,
    /// The wireless networks in reach.
    Wifi,
    /// The tunnels the machine has settings for.
    Vpn,
    /// The devices the adapter can reach.
    Bluetooth
}
