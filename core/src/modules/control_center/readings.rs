//! What the control centre knows, for anything that only wants to read it.
//!
//! The centre owns four services because it is where the user works them: the
//! sound, the link, the radio and the backlight are switched from one window,
//! so one module holds them. Everything else on the bar that wants to *say*
//! what they report — the canvas above all — has no business reaching into
//! that window, and no business starting a second copy of a service either.
//!
//! So the readings are handed out here, and only the readings: what comes back
//! is what the service last heard, never a handle to work it with. A caller
//! that has nothing to show gets [`None`], which is the same answer as a
//! machine without the hardware.

use super::ControlCenter;
use crate::services::{
    audio::AudioData, bluetooth::BluetoothData, brightness::BrightnessData, network::NetworkData
};

impl ControlCenter {
    /// What the sound server last reported, on a session that has one.
    #[must_use]
    pub fn audio_readings(&self) -> Option<&AudioData> {
        self.audio.as_deref()
    }

    /// What the network backend last reported, on a session that has one.
    #[must_use]
    pub fn network_readings(&self) -> Option<&NetworkData> {
        self.network.as_deref()
    }

    /// What the bluetooth adapter last reported, on a machine that has one.
    #[must_use]
    pub fn bluetooth_readings(&self) -> Option<&BluetoothData> {
        self.bluetooth.as_deref()
    }

    /// What the backlight last reported, on a screen that has one.
    #[must_use]
    pub fn brightness_readings(&self) -> Option<&BrightnessData> {
        self.brightness.as_deref()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_centre_without_services_reports_nothing_rather_than_a_blank() {
        let centre = ControlCenter::default();

        assert!(centre.audio_readings().is_none());
        assert!(centre.network_readings().is_none());
        assert!(centre.bluetooth_readings().is_none());
        assert!(centre.brightness_readings().is_none());
    }
}
