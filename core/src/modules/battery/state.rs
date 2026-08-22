//! Message folding for the battery module: service events in, state out.

use log::warn;

use super::{Battery, BatteryData, Message, PowerProfile};
use crate::services::{
    ServiceEvent,
    upower::{BatteryData as UPowerBatteryData, UPowerEvent, UPowerService}
};

impl Battery {
    /// Registers module with event system
    /// Processes incoming messages from GUI layer
    ///
    /// `animated` decides whether the shown percentage dissolves into its
    /// replacement or swaps outright.
    pub fn update(&mut self, message: Message, animated: bool) {
        match message {
            Message::Event(event) => self.handle_service_event(*event)
        }

        if let Some(data) = &self.data {
            self.shown.set(format!("{}%", data.capacity), animated);
        }
    }

    /// Advances the dissolve of the shown percentage.
    pub fn tick_fade(&mut self, elapsed: std::time::Duration) -> bool {
        self.shown.advance(elapsed)
    }

    /// Whether the shown percentage is still dissolving.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.shown.is_animating()
    }

    fn handle_service_event(&mut self, event: ServiceEvent<UPowerService>) {
        match event {
            ServiceEvent::Init(service) => {
                if let Some(battery) = service.battery {
                    self.update_battery_data(battery, service.power_profile.into());
                }
            }
            ServiceEvent::Update(update) => match update {
                UPowerEvent::UpdateBattery(battery) => {
                    let profile = self
                        .data
                        .as_ref()
                        .map(|d| d.power_profile)
                        .unwrap_or_default();
                    self.update_battery_data(battery, profile);
                }
                UPowerEvent::NoBattery => {
                    self.data = None;
                }
                UPowerEvent::UpdatePowerProfile(profile) => {
                    if let Some(data) = &mut self.data {
                        data.power_profile = profile.into();
                    }
                }
            },
            ServiceEvent::Error(()) => {
                warn!("Failed to receive battery updates from UPower");
            }
        }
    }

    /// Folds a fresh `UPower` reading into the shown data.
    ///
    /// Battery events are not currently sent to the UI; low- and
    /// critical-battery notifications would hook in here once they exist.
    fn update_battery_data(
        &mut self,
        upower_data: UPowerBatteryData,
        power_profile: PowerProfile
    ) {
        let capacity = u8::try_from(upower_data.capacity.clamp(0, 100)).unwrap_or(100);
        let charging = matches!(
            upower_data.status,
            crate::services::upower::BatteryStatus::Charging(_)
        );

        let data = BatteryData::new(capacity, charging, None, power_profile).worn(
            upower_data.health,
            upower_data.cycles,
            upower_data.watts,
            upower_data.watt_hours
        );

        self.data = Some(data);
    }
}
