//! Service commands the control center spawns onto its runtime.

use event::{EventCommandParams, spawn_event_command};
use optional::{OptionalEventCommandParams, spawn_optional_event_command};

use super::{
    audio::AudioMessage,
    bluetooth::BluetoothMessage,
    brightness::BrightnessMessage,
    network::NetworkMessage,
    state::{ControlCenter, Message},
    upower::UPowerMessage
};
use crate::services::{
    audio::{AudioCommand, AudioService},
    bluetooth::{BluetoothCommand, BluetoothService},
    brightness::{BrightnessCommand, BrightnessService},
    network::{NetworkCommand, NetworkService},
    upower::{PowerProfileCommand, UPowerService}
};

mod event;
mod optional;

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
            event_ctor: |event| AudioMessage::Event(Box::new(event)),
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
            event_ctor: |event| BrightnessMessage::Event(Box::new(event)),
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
            event_ctor: |event| NetworkMessage::Event(Box::new(event)),
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
            event_ctor: |event| BluetoothMessage::Event(Box::new(event)),
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
            event_ctor: |event| UPowerMessage::Event(Box::new(event)),
            service_name: "upower"
        })
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    /// A command sent before registration wired the runtime, the sender and
    /// the service must be refused, not queued: there is nothing to run it
    /// on, nobody to report to and no state to run it against.
    #[test]
    fn every_command_is_refused_before_registration_wires_the_center() {
        let center = ControlCenter::default();

        assert!(!center.spawn_audio_command(AudioCommand::ToggleSinkMute));
        assert!(!center.spawn_brightness_command(BrightnessCommand::Refresh));
        assert!(!center.spawn_network_command(NetworkCommand::ToggleWiFi));
        assert!(!center.spawn_bluetooth_command(BluetoothCommand::Toggle));
        assert!(!center.spawn_upower_command(PowerProfileCommand::Toggle));
    }
}
