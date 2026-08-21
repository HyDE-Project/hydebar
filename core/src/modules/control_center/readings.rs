//! Temporary probe accessors.

use super::ControlCenter;
use crate::services::{
    audio::AudioData, bluetooth::BluetoothData, brightness::BrightnessData, network::NetworkData,
    upower::PowerProfile
};

impl ControlCenter {
    #[must_use]
    pub fn audio_data(&self) -> Option<&AudioData> {
        self.audio.as_deref()
    }

    #[must_use]
    pub fn brightness_data(&self) -> Option<&BrightnessData> {
        self.brightness.as_deref()
    }

    #[must_use]
    pub fn network_data(&self) -> Option<&NetworkData> {
        self.network.as_deref()
    }

    #[must_use]
    pub fn bluetooth_data(&self) -> Option<&BluetoothData> {
        self.bluetooth.as_deref()
    }

    #[must_use]
    pub fn power_profile(&self) -> Option<PowerProfile> {
        self.upower.as_ref().map(|service| service.power_profile)
    }
}
